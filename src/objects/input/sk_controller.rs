use crate::nodes::{
	input::{tip::Tip, InputMethod, InputType},
	spatial::Spatial,
};
use glam::Mat4;
use stardust_xr::{schemas::flat::Datamap, values::Transform};
use std::sync::{Arc, Weak};
use stereokit::{
	input::{ButtonState, Handed},
	StereoKit,
};

pub struct SkController {
	tip: Arc<InputMethod>,
	handed: Handed,
}
impl SkController {
	pub fn new(handed: Handed) -> Self {
		SkController {
			tip: InputMethod::new(
				Spatial::new(Weak::new(), None, Mat4::IDENTITY),
				InputType::Tip(Tip::default()),
			),
			handed,
		}
	}
	pub fn update(&mut self, sk: &StereoKit) {
		let controller = sk.input_controller(self.handed);
		*self.tip.enabled.lock() = controller.tracked.contains(ButtonState::Active);
		if *self.tip.enabled.lock() {
			self.tip.spatial.set_local_transform_components(
				None,
				Transform {
					position: Some(controller.pose.position),
					rotation: Some(controller.pose.orientation),
					scale: None,
				},
			);
		}
		let mut fbb = flexbuffers::Builder::default();
		let mut map = fbb.start_map();
		map.push("select", controller.trigger);
		map.push("grab", controller.grip);
		map.end_map();
		*self.tip.datamap.lock() = Datamap::new(fbb.take_buffer()).ok();
	}
}
