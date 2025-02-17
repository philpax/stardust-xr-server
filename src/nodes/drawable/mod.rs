pub mod model;
pub mod text;

use super::Node;
use crate::core::client::Client;
use anyhow::Result;
use parking_lot::Mutex;
use serde::Deserialize;
use stardust_xr::schemas::flex::deserialize;
use std::{path::PathBuf, sync::Arc};
use stereokit::{lifecycle::DrawContext, texture::Texture, StereoKit};

pub fn create_interface(client: &Arc<Client>) {
	let node = Node::create(client, "", "drawable", false);
	node.add_local_signal("createModel", model::create_flex);
	node.add_local_signal("createText", text::create_flex);
	node.add_local_signal("setSkyFile", set_sky_file_flex);
	node.add_to_scenegraph();
}

pub fn draw(sk: &mut StereoKit, draw_ctx: &DrawContext) {
	model::draw_all(sk, draw_ctx);
	text::draw_all(sk, draw_ctx);

	let new_skytex = QUEUED_SKYTEX.lock().take();
	let mut new_skylight = QUEUED_SKYLIGHT.lock().take();
	let same_file = new_skytex == new_skylight;

	if let Some(skytex) = new_skytex {
		if let Some((skytex, skylight)) =
			Texture::from_cubemap_equirectangular(sk, &skytex, true, i32::MAX)
		{
			sk.set_skytex(&skytex);
			if same_file {
				sk.set_skylight(&skylight);
				new_skylight = None;
			}
		}
	}
	if let Some(skylight) = new_skylight {
		if let Some((_, skylight)) =
			Texture::from_cubemap_equirectangular(sk, &skylight, true, i32::MAX)
		{
			sk.set_skylight(&skylight);
		}
	}
}

static QUEUED_SKYLIGHT: Mutex<Option<PathBuf>> = Mutex::new(None);
static QUEUED_SKYTEX: Mutex<Option<PathBuf>> = Mutex::new(None);

fn set_sky_file_flex(_node: &Node, _calling_client: Arc<Client>, data: &[u8]) -> Result<()> {
	#[derive(Deserialize)]
	struct SkyFileInfo {
		path: PathBuf,
		skytex: Option<bool>,
		skylight: Option<bool>,
	}
	let info: SkyFileInfo = deserialize(data)?;
	info.path.metadata()?;
	if info.skytex.unwrap_or_default() {
		QUEUED_SKYTEX.lock().replace(info.path.clone());
	}
	if info.skylight.unwrap_or_default() {
		QUEUED_SKYLIGHT.lock().replace(info.path);
	}

	Ok(())
}
