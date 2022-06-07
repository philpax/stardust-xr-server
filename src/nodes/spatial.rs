use super::core::Node;
use crate::core::client::Client;
use anyhow::{anyhow, bail, ensure, Result};
use glam::Mat4;
use libstardustxr::{flex_to_quat, flex_to_vec3};
use rccell::{RcCell, WeakCell};
use std::cell::Cell;
use std::rc::Rc;

pub struct Spatial<'a> {
	node: WeakCell<Node<'a>>,
	parent: WeakCell<Node<'a>>,
	transform: Cell<Mat4>,
}

impl<'a> Spatial<'a> {
	pub fn add_to(
		node: RcCell<Node<'a>>,
		parent: WeakCell<Node<'a>>,
		transform: Mat4,
	) -> Result<()> {
		if node.borrow_mut().spatial.is_none() {
			bail!("Node already has a Spatial aspect!");
		}
		let spatial = Spatial {
			node: node.downgrade(),
			parent,
			transform: Cell::new(transform),
		};
		node.borrow_mut()
			.add_local_signal("setTransform", Spatial::set_transform_flex);
		node.borrow_mut().spatial = Some(Rc::new(spatial));
		Ok(())
	}

	pub fn set_transform_flex(node: &Node, calling_client: Rc<Client>, data: &[u8]) -> Result<()> {
		let root = flexbuffers::Reader::get_root(data)?;
		let flex_vec = root.get_vector()?;
		let client = node
			.get_client()
			.ok_or_else(|| anyhow!("Node somehow has no client"))?;
		let other_spatial = calling_client
			.get_scenegraph()
			.get_node(flex_vec.idx(0).as_str())
			.ok_or_else(|| anyhow!("Other spatial node not found"))?;
		ensure!(
			other_spatial.borrow().spatial.is_some(),
			"Node is not a Spatial!"
		);
		let pos = flex_to_vec3!(flex_vec.idx(1));
		let rot = flex_to_quat!(flex_vec.idx(2));
		let scl = flex_to_vec3!(flex_vec.idx(3));
		node.spatial
			.as_ref()
			.unwrap()
			.set_transform_components(other_spatial, pos, rot, scl);
		Ok(())
	}

	pub fn local_transform(&self) -> Mat4 {
		self.transform.get()
	}
	pub fn global_transform(&self) -> Mat4 {
		match self.parent.upgrade() {
			Some(value) => {
				value.borrow().spatial.as_ref().unwrap().global_transform() * self.transform.get()
			}
			None => self.transform.get(),
		}
	}

	pub fn set_transform_components(
		&self,
		relative_space: RcCell<Node>,
		pos: Option<mint::Vector3<f32>>,
		rot: Option<mint::Quaternion<f32>>,
		scl: Option<mint::Vector3<f32>>,
	) {
		todo!()
	}

	// pub fn relative_transform(&self, space: WeakCell<Spatial>) {}
}

pub fn create_interface(client: Rc<Client>) {
	let mut node = Node::create(Rc::downgrade(&client), "", "spatial", false);
	node.add_local_signal("createSpatial", create_spatial_flex);
	client.get_scenegraph().add_node(node);
}

pub fn create_spatial_flex(_node: &Node, calling_client: Rc<Client>, data: &[u8]) -> Result<()> {
	let root = flexbuffers::Reader::get_root(data)?;
	let flex_vec = root.get_vector()?;
	let spatial = Node::create(
		Rc::downgrade(&calling_client),
		"/spatial",
		flex_vec.idx(0).get_str()?,
		true,
	);
	let transform = Mat4::from_scale_rotation_translation(
		flex_to_vec3!(flex_vec.idx(4))
			.ok_or_else(|| anyhow!("Scale not found"))?
			.into(),
		flex_to_quat!(flex_vec.idx(3))
			.ok_or_else(|| anyhow!("Rotation not found"))?
			.into(),
		flex_to_vec3!(flex_vec.idx(2))
			.ok_or_else(|| anyhow!("Position not found"))?
			.into(),
	);
	let spatial_rc = calling_client.get_scenegraph().add_node(spatial);
	Spatial::add_to(spatial_rc, WeakCell::new(), transform)?;
	Ok(())
}
