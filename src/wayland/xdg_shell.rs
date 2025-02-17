use std::sync::Arc;

use crate::nodes::Node;

use super::{panel_item::PanelItem, state::WaylandState, surface::CoreSurface};
use smithay::{
	delegate_xdg_shell,
	reexports::{
		wayland_protocols::xdg::{
			decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode,
			shell::server::xdg_toplevel::State,
		},
		wayland_server::protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
	},
	utils::Serial,
	wayland::{
		compositor,
		shell::xdg::{
			Configure, PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler,
			XdgShellState,
		},
	},
};

impl XdgShellHandler for WaylandState {
	fn xdg_shell_state(&mut self) -> &mut XdgShellState {
		&mut self.xdg_shell_state
	}

	fn new_toplevel(&mut self, surface: ToplevelSurface) {
		self.output
			.enter(&self.display_handle, surface.wl_surface());
		surface.with_pending_state(|state| {
			state.states.set(State::Maximized);
			state.states.set(State::Activated);
			state.decoration_mode = Some(Mode::ServerSide);
		});
		surface.send_configure();
	}
	fn ack_configure(&mut self, surface: WlSurface, configure: Configure) {
		match configure {
			Configure::Toplevel(config) => {
				if let Some(size) = config.state.size {
					compositor::with_states(&surface, |data| {
						if let Some(panel_node) = data.data_map.get::<Arc<Node>>() {
							if let Some(core_surface) = data.data_map.get::<Arc<CoreSurface>>() {
								let panel_item = PanelItem::from_node(panel_node);
								let has_data = core_surface
									.with_data(|data| {
										data.size.x = size.w as u32;
										data.size.y = size.h as u32;
									})
									.is_some();
								if has_data {
									panel_item.resize();
								}
							}
						}
					})
				}
			}
			Configure::Popup(_) => (),
		}
	}

	fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {}
	fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {}
}
delegate_xdg_shell!(WaylandState);
