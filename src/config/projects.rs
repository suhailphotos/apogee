// src/config/projects.rs
#[derive(Debug, Default, serde::Deserialize)]
pub struct ProjectsConfig {
	#[serde(default)]
	pub managed: Vec<String>,

	#[serde(default)]
	pub houdini_packages: Vec<String>,
}
