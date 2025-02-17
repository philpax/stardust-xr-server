use crate::wayland::seat::SeatData;
use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use slog::Logger;
use smithay::{
	delegate_output, delegate_shm,
	output::{Output, Scale, Subpixel},
	reexports::{
		wayland_protocols_misc::server_decoration::server::org_kde_kwin_server_decoration_manager::Mode,
		wayland_server::{
			backend::{ClientData, ClientId, DisconnectReason},
			protocol::wl_data_device_manager::WlDataDeviceManager,
			Display, DisplayHandle,
		},
	},
	utils::Size,
	wayland::{
		buffer::BufferHandler,
		compositor::CompositorState,
		output::OutputManagerState,
		shell::{
			kde::decoration::KdeDecorationState,
			xdg::{decoration::XdgDecorationState, XdgShellState},
		},
		shm::{ShmHandler, ShmState},
		xdg_activation::XdgActivationState,
	},
};
use std::sync::{Arc, Weak};

pub struct ClientState;
impl ClientData for ClientState {
	fn initialized(&self, client_id: ClientId) {
		println!("Wayland client {:?} connected", client_id);
	}

	fn disconnected(&self, client_id: ClientId, reason: DisconnectReason) {
		println!(
			"Wayland client {:?} disconnected because {:#?}",
			client_id, reason
		);
	}
}

pub struct WaylandState {
	pub weak_ref: Weak<Mutex<WaylandState>>,
	pub display: Arc<Mutex<Display<WaylandState>>>,
	pub display_handle: DisplayHandle,

	pub compositor_state: CompositorState,
	pub xdg_activation_state: XdgActivationState,
	pub xdg_decoration_state: XdgDecorationState,
	pub kde_decoration_state: KdeDecorationState,
	pub xdg_shell_state: XdgShellState,
	pub shm_state: ShmState,
	pub output_manager_state: OutputManagerState,
	pub output: Output,
	pub seats: FxHashMap<ClientId, SeatData>,
}

impl WaylandState {
	pub fn new(
		log: Logger,
		display: Arc<Mutex<Display<WaylandState>>>,
		display_handle: DisplayHandle,
	) -> Arc<Mutex<Self>> {
		let compositor_state = CompositorState::new::<Self, _>(&display_handle, log.clone());
		let xdg_activation_state = XdgActivationState::new::<Self, _>(&display_handle, log.clone());
		let xdg_shell_state = XdgShellState::new::<Self, _>(&display_handle, log.clone());
		let xdg_decoration_state = XdgDecorationState::new::<Self, _>(&display_handle, log.clone());
		let kde_decoration_state =
			KdeDecorationState::new::<Self, _>(&display_handle, Mode::Server, log.clone());
		let shm_state = ShmState::new::<Self, _>(&display_handle, vec![], log.clone());
		let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&display_handle);
		let output = Output::new(
			"1x".to_owned(),
			smithay::output::PhysicalProperties {
				size: Size::default(),
				subpixel: Subpixel::None,
				make: "Virtual XR Display".to_owned(),
				model: "Your Headset Name Here".to_owned(),
			},
			log.clone(),
		);
		let _global = output.create_global::<Self>(&display_handle);
		output.change_current_state(None, None, Some(Scale::Integer(2)), None);
		display_handle.create_global::<Self, WlDataDeviceManager, _>(3, ());

		println!("Init Wayland compositor");

		Arc::new_cyclic(|weak| {
			Mutex::new(WaylandState {
				weak_ref: weak.clone(),
				display,
				display_handle,

				compositor_state,
				xdg_activation_state,
				xdg_decoration_state,
				kde_decoration_state,
				xdg_shell_state,
				shm_state,
				output_manager_state,
				output,
				seats: FxHashMap::default(),
			})
		})
	}

	pub fn new_client(&mut self, client: ClientId, dh: &DisplayHandle) {
		let seat_data = SeatData::new(dh, client.clone());
		self.seats.insert(client, seat_data);
	}
}
impl Drop for WaylandState {
	fn drop(&mut self) {
		println!("Cleanly shut down the Wayland compositor");
	}
}
impl BufferHandler for WaylandState {
	fn buffer_destroyed(
		&mut self,
		_buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
	) {
	}
}
impl ShmHandler for WaylandState {
	fn shm_state(&self) -> &smithay::wayland::shm::ShmState {
		&self.shm_state
	}
}
delegate_shm!(WaylandState);
delegate_output!(WaylandState);
