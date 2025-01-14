use super::{
	seat::{KeyboardInfo, SeatData},
	surface::CoreSurface,
};
use crate::{
	core::{
		client::{Client, INTERNAL_CLIENT},
		registry::Registry,
	},
	nodes::{
		items::{register_item_ui_flex, Item, ItemSpecialization, ItemType, TypeInfo},
		spatial::Spatial,
		Node,
	},
};
use anyhow::{anyhow, bail, Result};
use glam::Mat4;
use lazy_static::lazy_static;
use mint::Vector2;
use nanoid::nanoid;
use serde::Deserialize;
use smithay::{
	reexports::wayland_server::protocol::wl_pointer::{Axis, ButtonState},
	utils::Size,
	wayland::{compositor::SurfaceData, shell::xdg::XdgToplevelSurfaceData},
};
use stardust_xr::schemas::flex::{deserialize, serialize};
use std::sync::{Arc, Weak};
use xkbcommon::xkb::{self, ffi::XKB_KEYMAP_FORMAT_TEXT_V1, Keymap};

lazy_static! {
	static ref ITEM_TYPE_INFO_PANEL: TypeInfo = TypeInfo {
		type_name: "panel",
		aliased_local_signals: vec![
			"applySurfaceMaterial",
			"applyCursorMaterial",
			"pointerDeactivate",
			"pointerScroll",
			"pointerButton",
			"pointerMotion",
			"keyboardSetActive",
			"keyboardSetKeyState",
			"keyboardSetModifiers",
			"resize",
			"close",
		],
		aliased_local_methods: vec![],
		aliased_remote_signals: vec!["resize", "setCursor",],
		aliased_remote_methods: vec![],
		ui: Default::default(),
		items: Registry::new(),
		acceptors: Registry::new(),
	};
}

pub struct PanelItem {
	node: Weak<Node>,
	core_surface: Weak<CoreSurface>,
	seat_data: SeatData,
}
impl PanelItem {
	pub fn create(core_surface: &Arc<CoreSurface>, seat_data: SeatData) -> Arc<Node> {
		let node = Arc::new(Node::create(
			&INTERNAL_CLIENT,
			"/item/panel/item",
			&nanoid!(),
			true,
		));
		Spatial::add_to(&node, None, Mat4::IDENTITY).unwrap();

		let specialization = ItemType::Panel(PanelItem {
			node: Arc::downgrade(&node),
			core_surface: Arc::downgrade(core_surface),
			seat_data,
		});
		let item = Item::add_to(&node, &ITEM_TYPE_INFO_PANEL, specialization);
		if let ItemType::Panel(panel) = &item.specialization {
			let _ = panel.seat_data.panel_item.set(Arc::downgrade(&item));
		}

		node.add_local_signal(
			"applySurfaceMaterial",
			PanelItem::apply_surface_material_flex,
		);
		node.add_local_signal("applyCursorMaterial", PanelItem::apply_cursor_material_flex);
		node.add_local_signal("pointerDeactivate", PanelItem::pointer_deactivate_flex);
		node.add_local_signal("pointerScroll", PanelItem::pointer_scroll_flex);
		node.add_local_signal("pointerButton", PanelItem::pointer_button_flex);
		node.add_local_signal("pointerMotion", PanelItem::pointer_motion_flex);
		node.add_local_signal(
			"keyboardActivateString",
			PanelItem::keyboard_activate_string_flex,
		);
		node.add_local_signal(
			"keyboardActivateNames",
			PanelItem::keyboard_activate_names_flex,
		);
		node.add_local_signal("keyboardDeactivate", PanelItem::keyboard_deactivate_flex);
		node.add_local_signal("keyboardKeyState", PanelItem::keyboard_key_state_flex);
		node.add_local_signal("resize", PanelItem::resize_flex);
		node
	}

	pub fn from_node(node: &Node) -> &PanelItem {
		match &node.item.get().unwrap().specialization {
			ItemType::Panel(panel_item) => panel_item,
			_ => unreachable!(),
		}
	}

	fn apply_surface_material_flex(
		node: &Node,
		calling_client: Arc<Client>,
		data: &[u8],
	) -> Result<()> {
		#[derive(Deserialize)]
		struct SurfaceMaterialInfo<'a> {
			model_path: &'a str,
			idx: u32,
		}
		let info: SurfaceMaterialInfo = deserialize(data)?;
		let model_node = calling_client
			.scenegraph
			.get_node(info.model_path)
			.ok_or_else(|| anyhow!("Model node not found"))?;
		let model = model_node
			.model
			.get()
			.ok_or_else(|| anyhow!("Node is not a model"))?;

		if let ItemType::Panel(panel_item) = &node.item.get().unwrap().specialization {
			if let Some(core_surface) = panel_item.core_surface.upgrade() {
				core_surface.apply_material(model.clone(), info.idx);
			}
		}

		Ok(())
	}

	fn apply_cursor_material_flex(
		node: &Node,
		calling_client: Arc<Client>,
		data: &[u8],
	) -> Result<()> {
		#[derive(Deserialize)]
		struct SurfaceMaterialInfo<'a> {
			model_path: &'a str,
			idx: u32,
		}
		let info: SurfaceMaterialInfo = deserialize(data)?;
		let model_node = calling_client
			.scenegraph
			.get_node(info.model_path)
			.ok_or_else(|| anyhow!("Model node not found"))?;
		let model = model_node
			.model
			.get()
			.ok_or_else(|| anyhow!("Node is not a model"))?;

		if let ItemType::Panel(panel_item) = &node.item.get().unwrap().specialization {
			if let Some(cursor) = &*panel_item.seat_data.cursor.lock() {
				if let Some(core_surface) = cursor.lock().core_surface.upgrade() {
					core_surface.apply_material(model.clone(), info.idx);
				}
			}
		}

		Ok(())
	}

	pub fn on_mapped(
		core_surface: &Arc<CoreSurface>,
		surface_data: &SurfaceData,
		seat_data: SeatData,
	) {
		if surface_data
			.data_map
			.get::<XdgToplevelSurfaceData>()
			.is_some()
		{
			surface_data
				.data_map
				.insert_if_missing_threadsafe(|| PanelItem::create(core_surface, seat_data));
		}
	}

	pub fn if_mapped(_core_surface: &Arc<CoreSurface>, surface_data: &SurfaceData) {
		if let Some(panel_node) = surface_data.data_map.get::<Arc<Node>>() {
			let panel_item = PanelItem::from_node(panel_node);

			// core_surface.with_data(|core_surface_data| {
			// 	panel_item.resize();
			// });

			panel_item.set_cursor();
		}
	}

	pub fn resize(&self) {
		self.core_surface.upgrade().unwrap().with_data(|data| {
			let _ = self
				.node
				.upgrade()
				.unwrap()
				.send_remote_signal("resize", &serialize(data.size).unwrap());
		});
	}

	pub fn set_cursor(&self) {
		let mut cursor_changed = self.seat_data.cursor_changed.lock();
		if !*cursor_changed {
			return;
		}
		let mut data = serialize(()).unwrap();

		if let Some(cursor) = &*self.seat_data.cursor.lock() {
			let cursor = cursor.lock();
			if let Some(core_surface) = cursor.core_surface.upgrade() {
				if let Some(mapped_data) = &*core_surface.mapped_data.lock() {
					data = serialize((mapped_data.size, cursor.hotspot)).unwrap();
				} else {
					return;
				};
			} else {
				return;
			}
		}

		let _ = self
			.node
			.upgrade()
			.unwrap()
			.send_remote_signal("setCursor", &data);
		*cursor_changed = false;
	}

	fn pointer_deactivate_flex(
		node: &Node,
		_calling_client: Arc<Client>,
		_data: &[u8],
	) -> Result<()> {
		let panel_item = PanelItem::from_node(node);
		if *panel_item.seat_data.pointer_active.lock() {
			if let Some(core_surface) = panel_item.core_surface.upgrade() {
				if let Some(pointer) = panel_item.seat_data.pointer() {
					pointer.leave(0, &core_surface.wl_surface());
					*panel_item.seat_data.pointer_active.lock() = false;
					pointer.frame();
					core_surface.flush_clients();
				}
			}
		}

		Ok(())
	}

	fn pointer_motion_flex(node: &Node, _calling_client: Arc<Client>, data: &[u8]) -> Result<()> {
		if let ItemType::Panel(panel_item) = &node.item.get().unwrap().specialization {
			if let Some(pointer) = panel_item.seat_data.pointer() {
				if let Some(core_surface) = panel_item.core_surface.upgrade() {
					if let Some(size) = core_surface.with_data(|data| data.size) {
						let mut position: Vector2<f64> = deserialize(data)?;
						position.x = position.x.clamp(0.0, size.x as f64);
						position.y = position.y.clamp(0.0, size.y as f64);
						let mut pointer_active = panel_item.seat_data.pointer_active.lock();
						if *pointer_active {
							pointer.motion(0, position.x, position.y);
						} else {
							pointer.enter(0, &core_surface.wl_surface(), position.x, position.y);
							*pointer_active = true;
						}
						pointer.frame();
						core_surface.flush_clients();
					}
				}
			}
		}

		Ok(())
	}

	fn pointer_button_flex(node: &Node, _calling_client: Arc<Client>, data: &[u8]) -> Result<()> {
		if let ItemType::Panel(panel_item) = &node.item.get().unwrap().specialization {
			if let Some(pointer) = panel_item.seat_data.pointer() {
				if *panel_item.seat_data.pointer_active.lock() {
					if let Some(core_surface) = panel_item.core_surface.upgrade() {
						let (button, state): (u32, u32) = deserialize(data)?;
						pointer.button(
							0,
							0,
							button,
							match state {
								0 => ButtonState::Released,
								1 => ButtonState::Pressed,
								_ => {
									bail!("Button state is out of bounds")
								}
							},
						);
						pointer.frame();
						core_surface.flush_clients();
					}
				}
			}
		}

		Ok(())
	}

	fn pointer_scroll_flex(node: &Node, _calling_client: Arc<Client>, data: &[u8]) -> Result<()> {
		#[derive(Deserialize)]
		struct PointerScrollArgs {
			axis_continuous: Vector2<f32>,
			axis_discrete: Option<Vector2<f32>>,
		}
		let args: PointerScrollArgs = deserialize(data)?;
		if let ItemType::Panel(panel_item) = &node.item.get().unwrap().specialization {
			if let Some(pointer) = panel_item.seat_data.pointer() {
				if *panel_item.seat_data.pointer_active.lock() {
					if let Some(core_surface) = panel_item.core_surface.upgrade() {
						let flex = flexbuffers::Reader::get_root(data)?;
						if flex.flexbuffer_type().is_null() {
							pointer.axis_stop(0, Axis::HorizontalScroll);
							pointer.axis_stop(0, Axis::VerticalScroll);
						} else {
							pointer.axis(0, Axis::HorizontalScroll, args.axis_continuous.x as f64);
							pointer.axis(0, Axis::VerticalScroll, args.axis_continuous.y as f64);
							if let Some(axis_discrete_vec) = args.axis_discrete {
								pointer.axis_discrete(
									Axis::HorizontalScroll,
									axis_discrete_vec.x as i32,
								);
								pointer.axis_discrete(
									Axis::VerticalScroll,
									axis_discrete_vec.y as i32,
								);
							}
						}
						pointer.frame();
						core_surface.flush_clients();
					}
				}
			}
		}

		Ok(())
	}

	fn keyboard_activate_string_flex(
		node: &Node,
		_calling_client: Arc<Client>,
		data: &[u8],
	) -> Result<()> {
		let context = xkb::Context::new(0);
		let keymap =
			Keymap::new_from_string(&context, deserialize(data)?, XKB_KEYMAP_FORMAT_TEXT_V1, 0)
				.ok_or_else(|| anyhow!("Keymap is not valid"))?;

		PanelItem::keyboard_activate_flex(node, &keymap)
	}

	fn keyboard_activate_names_flex(
		node: &Node,
		_calling_client: Arc<Client>,
		data: &[u8],
	) -> Result<()> {
		#[derive(Deserialize)]
		struct Names<'a> {
			rules: &'a str,
			model: &'a str,
			layout: &'a str,
			variant: &'a str,
			options: Option<String>,
		}
		let names: Names = deserialize(data)?;
		let context = xkb::Context::new(0);
		let keymap = Keymap::new_from_names(
			&context,
			names.rules,
			names.model,
			names.layout,
			names.variant,
			names.options,
			XKB_KEYMAP_FORMAT_TEXT_V1,
		)
		.ok_or_else(|| anyhow!("Keymap is not valid"))?;

		PanelItem::keyboard_activate_flex(node, &keymap)
	}

	fn keyboard_activate_flex(node: &Node, keymap: &Keymap) -> Result<()> {
		if let ItemType::Panel(panel_item) = &node.item.get().unwrap().specialization {
			if let Some(keyboard) = panel_item.seat_data.keyboard() {
				if let Some(core_surface) = panel_item.core_surface.upgrade() {
					let mut keyboard_info = panel_item.seat_data.keyboard_info.lock();
					if keyboard_info.is_none() {
						keyboard.enter(0, &core_surface.wl_surface(), vec![]);
						keyboard.repeat_info(0, 0);
					}
					keyboard_info.replace(KeyboardInfo::new(keymap));
					keyboard_info.as_ref().unwrap().keymap.send(keyboard)?;
				}
			}
		}

		Ok(())
	}

	fn keyboard_deactivate_flex(
		node: &Node,
		_calling_client: Arc<Client>,
		_data: &[u8],
	) -> Result<()> {
		if let ItemType::Panel(panel_item) = &node.item.get().unwrap().specialization {
			if let Some(keyboard) = panel_item.seat_data.keyboard() {
				if let Some(core_surface) = panel_item.core_surface.upgrade() {
					let mut keyboard_info = panel_item.seat_data.keyboard_info.lock();
					if keyboard_info.is_some() {
						keyboard.leave(0, &core_surface.wl_surface());
						*keyboard_info = None;
					}
				}
			}
		}

		Ok(())
	}

	fn keyboard_key_state_flex(
		node: &Node,
		_calling_client: Arc<Client>,
		data: &[u8],
	) -> Result<()> {
		if let ItemType::Panel(panel_item) = &node.item.get().unwrap().specialization {
			if let Some(keyboard) = panel_item.seat_data.keyboard() {
				let mut keyboard_info = panel_item.seat_data.keyboard_info.lock();
				if let Some(keyboard_info) = &mut *keyboard_info {
					let (key, state): (u32, u32) = deserialize(data)?;
					keyboard_info.process(key, state, keyboard)?;
				}
			}
		}

		Ok(())
	}

	fn resize_flex(node: &Node, _calling_client: Arc<Client>, data: &[u8]) -> Result<()> {
		if let ItemType::Panel(panel_item) = &node.item.get().unwrap().specialization {
			if let Some(core_surface) = panel_item.core_surface.upgrade() {
				let size: Vector2<u32> = deserialize(data)?;

				let toplevel_surface = core_surface
					.wayland_state()
					.lock()
					.xdg_shell_state
					.toplevel_surfaces(|surfaces| {
						surfaces
							.iter()
							.find(|surf| surf.wl_surface().clone() == core_surface.wl_surface())
							.cloned()
					});

				if let Some(toplevel_surface) = toplevel_surface {
					let mut size_set = false;
					toplevel_surface.with_pending_state(|state| {
						state.size = Some(Size::default());
						state.size.as_mut().unwrap().w = size.x as i32;
						state.size.as_mut().unwrap().h = size.y as i32;
						size_set = true;
					});
					if size_set {
						toplevel_surface.send_configure();
					}
				}
			}
		}
		Ok(())
	}
}
impl ItemSpecialization for PanelItem {
	fn serialize_start_data(&self, id: &str) -> Vec<u8> {
		// Panel size
		let panel_size = self
			.core_surface
			.upgrade()
			.unwrap()
			.with_data(|data| data.size);

		let cursor_lock = (*self.seat_data.cursor.lock()).clone();
		let cursor_size = cursor_lock
			.clone()
			.and_then(|cursor| cursor.lock().core_surface.upgrade())
			.and_then(|surf| surf.with_data(|data| data.size));
		let cursor_hotspot = cursor_lock.map(|cursor| cursor.lock().hotspot);

		serialize((id, (panel_size, cursor_size.zip(cursor_hotspot)))).unwrap()
	}
}

pub fn register_panel_item_ui_flex(
	_node: &Node,
	calling_client: Arc<Client>,
	_data: &[u8],
) -> Result<()> {
	register_item_ui_flex(calling_client, &ITEM_TYPE_INFO_PANEL)
}
