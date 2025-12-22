use anyhow::Result;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

fn default_priority() -> i32 {
    1000
}

pub fn default_config_path() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("apogee").join("config.toml");
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("apogee")
            .join("config.toml");
    }
    PathBuf::from("apogee/config.toml")
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub apogee: ApogeeMeta,

    #[serde(default)]
    pub modules: ModulesRoot,

    #[serde(default)]
    pub global: GlobalConfig,
}

impl Config {
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let cfg: Self = toml::from_str(&text)?;
        Ok(cfg)
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let schema = self.apogee.schema_version;
        let default_shell = self.apogee.default_shell;

        let platforms = if self.apogee.platforms.is_empty() {
            "<none>".to_string()
        } else {
            self.apogee
                .platforms
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };

        let knobs = format!(
            "cloud={} apps={} hooks={} templates={}",
            self.modules.enable_cloud,
            self.modules.enable_apps,
            self.modules.enable_hooks,
            self.modules.enable_templates
        );

        let cloud_items = self.modules.cloud.items.len();
        let apps_items = self.modules.apps.items.len();
        let hooks_items = self.modules.hooks.items.len();
        let templates_items = self.modules.templates.items.len();

        write!(
            f,
            "apogee config\n\
               - schema_version: {schema}\n\
               - default_shell: {default_shell}\n\
               - platforms: {platforms}\n\
               - modules: {knobs}\n\
               - cloud: enabled={} items={}\n\
               - apps: enabled={} items={}\n\
               - hooks: enabled={} items={}\n\
               - templates: enabled={} items={}",
            self.modules.cloud.enabled,
            cloud_items,
            self.modules.apps.enabled,
            apps_items,
            self.modules.hooks.enabled,
            hooks_items,
            self.modules.templates.enabled,
            templates_items,
        )
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct SourceEmit {
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApogeeMeta {
    pub schema_version: u32,

    #[serde(default = "default_shell")]
    pub default_shell: Shell,

    #[serde(default)]
    pub platforms: Vec<Platform>,

    /// Default: "{config_dir}/.env" (applied by runtime builder if None)
    #[serde(default)]
    pub env_file: Option<String>,

    #[serde(default)]
    pub secrets_file: Option<String>,

    #[serde(default)]
    pub bootstrap: Option<BootstrapConfig>,
}

#[derive(Debug, Default, Deserialize)]
pub struct BootstrapConfig {
    #[serde(default)]
    pub defaults: BootstrapDefaults,

    #[serde(default)]
    pub secrets: BootstrapSecrets,
}

#[derive(Debug, Default, Deserialize)]
pub struct BootstrapDefaults {
    #[serde(default)]
    pub env: EnvMap,
}

#[derive(Debug, Deserialize)]
pub struct BootstrapSecrets {
    #[serde(default = "default_secrets_strategy")]
    pub strategy: SecretsStrategy,
}

impl Default for BootstrapSecrets {
    fn default() -> Self {
        Self {
            strategy: default_secrets_strategy(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretsStrategy {
    FillMissing,
    Override,
}

fn default_secrets_strategy() -> SecretsStrategy {
    SecretsStrategy::FillMissing
}

fn default_shell() -> Shell {
    Shell::Zsh
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Shell {
    Zsh,
    Bash,
    Fish,
    Pwsh,
}

impl Shell {
    /// Parse common shell strings (case-insensitive).
    /// Accepts: zsh, bash, fish, pwsh, powershell.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        let s = s.to_ascii_lowercase();
        match s.as_str() {
            "zsh" => Some(Shell::Zsh),
            "bash" => Some(Shell::Bash),
            "fish" => Some(Shell::Fish),
            "pwsh" | "powershell" => Some(Shell::Pwsh),
            _ => None,
        }
    }
}

impl FromStr for Shell {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("invalid shell: {s}"))
    }
}

impl fmt::Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Shell::Zsh => "zsh",
            Shell::Bash => "bash",
            Shell::Fish => "fish",
            Shell::Pwsh => "pwsh",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Mac,
    Linux,
    Windows,
    Wsl,
    Other,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Platform::Mac => "mac",
            Platform::Linux => "linux",
            Platform::Windows => "windows",
            Platform::Wsl => "wsl",
            Platform::Other => "other",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct ModulesRoot {
    #[serde(default = "default_true")]
    pub enable_cloud: bool,

    #[serde(default = "default_true")]
    pub enable_apps: bool,

    #[serde(default = "default_true")]
    pub enable_hooks: bool,

    #[serde(default = "default_true")]
    pub enable_templates: bool,

    #[serde(default)]
    pub cloud: CloudModules,

    #[serde(default)]
    pub apps: AppModules,

    #[serde(default)]
    pub hooks: HooksModules,

    #[serde(default)]
    pub templates: TemplateModules,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Default, Deserialize)]
pub struct GlobalConfig {
    #[serde(default)]
    pub aliases: GlobalAliases,
}

#[derive(Debug, Default, Deserialize)]
pub struct GlobalAliases {
    #[serde(default)]
    pub platform: PlatformAliasMaps,

    #[serde(default)]
    pub shell: ShellAliasMaps,
}

#[derive(Debug, Default, Deserialize)]
pub struct PlatformAliasMaps {
    #[serde(default)]
    pub mac: AliasMap,
    #[serde(default)]
    pub linux: AliasMap,
    #[serde(default)]
    pub windows: AliasMap,
    #[serde(default)]
    pub wsl: AliasMap,
    #[serde(default)]
    pub other: AliasMap,
}

#[derive(Debug, Default, Deserialize)]
pub struct ShellAliasMaps {
    #[serde(default)]
    pub zsh: AliasMap,
    #[serde(default)]
    pub bash: AliasMap,
    #[serde(default)]
    pub fish: AliasMap,
    #[serde(default)]
    pub pwsh: AliasMap,
}

pub type AliasMap = BTreeMap<String, String>;
pub type EnvMap = BTreeMap<String, String>;

#[derive(Debug, Default, Deserialize)]
pub struct CloudModules {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(flatten, default)]
    pub items: BTreeMap<String, CloudModule>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CloudModule {
    pub enabled: bool,

    #[serde(default)]
    pub kind: Option<CloudKind>,

    #[serde(default = "default_priority")]
    pub priority: i32,

    #[serde(default)]
    pub requires: Vec<String>,

    #[serde(default)]
    pub platforms: Vec<Platform>,

    #[serde(default)]
    pub detect: DetectBlock,

    #[serde(default)]
    pub emit: EmitBlock,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloudKind {
    Storage,
    Service,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct AppModules {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(flatten, default)]
    pub items: BTreeMap<String, AppModule>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppModule {
    pub enabled: bool,

    #[serde(default)]
    pub kind: Option<AppKind>,

    #[serde(default = "default_priority")]
    pub priority: i32,

    #[serde(default)]
    pub requires: Vec<String>,

    #[serde(default)]
    pub platforms: Vec<Platform>,

    #[serde(default)]
    pub detect: DetectBlock,

    #[serde(default)]
    pub emit: EmitBlock,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppKind {
    Cli,
    Desktop,
}

#[derive(Debug, Default, Deserialize)]
pub struct HooksModules {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default)]
    pub items: Vec<HookItem>,
}

#[derive(Debug, Deserialize)]
pub struct HookItem {
    pub name: String,
    pub enabled: bool,

    #[serde(default)]
    pub platforms: Vec<Platform>,

    #[serde(default)]
    pub hosts: Vec<String>,

    #[serde(default)]
    pub shells: Vec<Shell>,

    pub script: String,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct DetectBlock {
    #[serde(default)]
    pub paths: PlatformAnyOf,

    #[serde(default)]
    pub files: PlatformAnyOf,

    #[serde(default)]
    pub commands: AnyOf,

    #[serde(default)]
    pub env: AnyOf,

    #[serde(default)]
    pub version: Option<VersionDetectSpec>,
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct VersionDetectSpec {
    #[serde(default)]
    pub all: Option<OneOrMany<VersionDetect>>,
    #[serde(default)]
    pub mac: Option<OneOrMany<VersionDetect>>,
    #[serde(default)]
    pub linux: Option<OneOrMany<VersionDetect>>,
    #[serde(default)]
    pub windows: Option<OneOrMany<VersionDetect>>,
    #[serde(default)]
    pub wsl: Option<OneOrMany<VersionDetect>>,
    #[serde(default)]
    pub other: Option<OneOrMany<VersionDetect>>,
}

impl VersionDetectSpec {
    pub fn for_platform(&self, p: Platform) -> Option<&OneOrMany<VersionDetect>> {
        let pick = match p {
            Platform::Mac => self.mac.as_ref(),
            Platform::Linux => self.linux.as_ref(),
            Platform::Windows => self.windows.as_ref(),
            Platform::Wsl => self.wsl.as_ref(),
            Platform::Other => self.other.as_ref(),
        };
        pick.or(self.all.as_ref())
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VersionDetect {
    Command {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        regex: Option<String>,
        #[serde(default = "default_version_capture")]
        capture: String,
    },

    PathRegex {
        regex: String,
        #[serde(default = "default_version_capture")]
        capture: String,
    },

    // macOS: read Info.plist key from Foo.app (handles binary plists via plutil/defaults)
    MacBundlePlist {
        path: String, // "{detect.path}" or "{detect.file}" or explicit
        key: String,  // "CFBundleShortVersionString" etc
        #[serde(default)]
        regex: Option<String>,
        #[serde(default = "default_version_capture")]
        capture: String,
    },

    // Windows: read ProductVersion/FileVersion from an exe/dll
    WindowsFileVersion {
        path: String, // "{detect.command_path}" or explicit
        #[serde(default)]
        field: Option<String>, // default "ProductVersion" (or "FileVersion")
        #[serde(default)]
        regex: Option<String>,
        #[serde(default = "default_version_capture")]
        capture: String,
    },

    // Linux/WSL: parse a key from a .desktop file (optional)
    LinuxDesktopFileKey {
        path: String, // path to .desktop
        #[serde(default)]
        section: Option<String>, // default "Desktop Entry"
        key: String,
        #[serde(default)]
        regex: Option<String>,
        #[serde(default = "default_version_capture")]
        capture: String,
    },
}

fn default_version_capture() -> String {
    "version".to_string()
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> OneOrMany<T> {
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        match self {
            OneOrMany::One(x) => std::slice::from_ref(x).iter(),
            OneOrMany::Many(xs) => xs.iter(),
        }
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct PlatformAnyOf {
    #[serde(default)]
    pub mac: AnyOf,
    #[serde(default)]
    pub linux: AnyOf,
    #[serde(default)]
    pub windows: AnyOf,
    #[serde(default)]
    pub wsl: AnyOf,
    #[serde(default)]
    pub other: AnyOf,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct AnyOf {
    #[serde(default)]
    pub any_of: Vec<String>,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct EmitBlock {
    #[serde(default)]
    pub env: EnvMap,

    #[serde(default)]
    pub env_derived: EnvMap,

    #[serde(default)]
    pub aliases: AliasMap,

    #[serde(default)]
    pub source: SourceEmit,

    #[serde(default)]
    pub functions: FunctionsEmit,

    #[serde(default)]
    pub paths: PathsEmit,

    #[serde(default)]
    pub init: Vec<EmitInit>,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct PathsEmit {
    #[serde(default)]
    pub prepend_if_exists: Vec<String>,

    #[serde(default)]
    pub append_if_exists: Vec<String>,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct FunctionsEmit {
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EmitInit {
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,

    // PowerShell quirk:
    // zoxide docs use: Invoke-Expression (& { (zoxide init powershell | Out-String) })
    // starship docs use: Invoke-Expression (&starship init powershell)
    #[serde(default)]
    pub pwsh_out_string: bool,
}

// ---------------- Templates Modules ----------------

#[derive(Debug, Default, Deserialize, Clone)]
pub struct TemplateModules {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(flatten, default)]
    pub items: BTreeMap<String, TemplateModule>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TemplateModule {
    pub enabled: bool,

    #[serde(default = "default_priority")]
    pub priority: i32,

    #[serde(default)]
    pub requires: Vec<String>,

    #[serde(default)]
    pub platforms: Vec<Platform>,

    /// Per-shell template file paths (user provides). `all` is a fallback.
    #[serde(default)]
    pub templates: TemplateFiles,

    /// Arbitrary data passed to the template.
    /// Use TOML tables/arrays; deserializes into a JSON-like value.
    #[serde(default)]
    pub data: JsonValue,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct TemplateFiles {
    #[serde(default)]
    pub all: Option<String>,

    #[serde(default)]
    pub zsh: Option<String>,
    #[serde(default)]
    pub bash: Option<String>,
    #[serde(default)]
    pub fish: Option<String>,
    #[serde(default)]
    pub pwsh: Option<String>,
}

impl TemplateFiles {
    pub fn for_shell(&self, shell: Shell) -> Option<&str> {
        let s = match shell {
            Shell::Zsh => self.zsh.as_deref(),
            Shell::Bash => self.bash.as_deref(),
            Shell::Fish => self.fish.as_deref(),
            Shell::Pwsh => self.pwsh.as_deref(),
        };
        s.or(self.all.as_deref())
    }
}
