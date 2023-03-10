use clap::Args;
use moni_core::{Meta, DEFAULT_METADATA_FILE};
use std::path::{Path, PathBuf};
use tracing::{debug, debug_span, error, info, Instrument};

/// Command Init
#[derive(Args, Debug)]
pub struct Init {
    /// The name of the project
    pub name: String,
    /// The template to use
    #[clap(long, default_value("rust-basic"))]
    pub template: Option<String>,
}

impl Init {
    pub async fn run(&self) {
        debug!("Init: {self:?}");
        // create dir by name
        if !Path::new(&self.name).exists() {
            std::fs::create_dir(&self.name).unwrap();
            info!("Created dir: {}", &self.name)
        }
        // create metadata
        let meta = self.create_metadata();
        // create project
        self.create_project(meta);
        info!("Created project {} success", &self.name);
    }

    fn create_metadata(&self) -> Meta {
        let metadata_file =
            PathBuf::from(&self.template.as_ref().unwrap()).join(DEFAULT_METADATA_FILE);
        let meta = crate::embed::TemplateAssets::get(metadata_file.to_str().unwrap());
        if meta.is_none() {
            panic!("Template {} is not valid", &self.template.as_ref().unwrap());
        }
        let mut meta = Meta::from_binary(&meta.unwrap().data).unwrap();
        meta.name = self.name.clone();
        meta.build = None;
        let metadata_file = PathBuf::from(&self.name).join(DEFAULT_METADATA_FILE);
        meta.to_file(metadata_file.to_str().unwrap()).unwrap();
        info!("Created metadata: {:?}", metadata_file);
        meta
    }

    fn create_cargo_toml(&self) {
        let template = self.template.as_ref().unwrap();
        let name = self.name.as_str();

        let toml_file = PathBuf::from(template).join("Cargo.toml");
        let toml_data = crate::embed::TemplateAssets::get(toml_file.to_str().unwrap());
        if toml_data.is_none() {
            panic!(
                "Template {} is not valid with rust Cargo.toml",
                &self.template.as_ref().unwrap()
            );
        }
        let target_file = PathBuf::from(&self.name).join("Cargo.toml");

        // replace cargo toml to correct deps
        let mut content = std::str::from_utf8(&toml_data.unwrap().data)
            .unwrap()
            .to_string();
        content = content.replace(template, name);
        content = content.replace(
            "path = \"../../moni-sdk\"",
            "git = \"https://github.com/fuxiaohei/moni-serverless\"",
        );
        std::fs::write(target_file.to_str().unwrap(), content).unwrap();

        info!("Created Cargo.toml: {:?}", target_file);
    }

    fn create_project(&self, meta: Meta) {
        // if rust project, copy Cargo.toml
        if meta.language == "rust" {
            self.create_cargo_toml();
        }

        // create src dir
        let src_dir = Path::new(&self.name).join("src");
        std::fs::create_dir_all(src_dir.parent().unwrap()).unwrap();

        // copy src files
        let tpl_dir = Path::new(&self.template.as_ref().unwrap()).join("src");
        crate::embed::TemplateAssets::iter().for_each(|t| {
            if t.starts_with(tpl_dir.to_str().unwrap()) {
                let src_path = Path::new(t.as_ref())
                    .strip_prefix(tpl_dir.to_str().unwrap())
                    .unwrap();
                let file = crate::embed::TemplateAssets::get(t.as_ref()).unwrap();
                let content = std::str::from_utf8(&file.data).unwrap().to_string();
                let target_path = src_dir.join(src_path);
                debug!("Created src: {:?}, {:?}", src_path, target_path);
                std::fs::create_dir_all(target_path.parent().unwrap()).unwrap();
                std::fs::write(target_path, content).unwrap();
            }
        });
    }
}

/// Command Build
#[derive(Args, Debug)]
pub struct Build {
    /// Set js engine wasm file
    #[clap(long)]
    pub js_engine: Option<String>,
}

impl Build {
    pub async fn run(&self) {
        debug!("Build: {self:?}");

        // read metadata from file
        let meta = Meta::from_file(DEFAULT_METADATA_FILE).expect("Project meta.toml not found");
        let arch = meta.get_arch();
        info!("Build arch: {}", arch);

        let target = meta.get_target();
        info!("Build target: {}", target);

        // call cargo to build wasm
        match meta.language.as_str() {
            "rust" => {
                moni_runtime::compile_rust(&arch, &target).expect("Build failed");
            }
            "js" => {
                moni_runtime::compile_js(&target, "src/index.js", self.js_engine.clone())
                    .expect("Build failed");
            }
            _ => {
                panic!("Unsupported language: {}", meta.language);
            }
        }

        // convert wasm module to component
        let output = meta.get_output();
        moni_runtime::convert_component(&target, Some(output)).expect("Convert failed");
    }
}

/// Command Serve
#[derive(Args, Debug)]
pub struct Serve {
    /// The port to listen on
    #[clap(long, default_value("127.0.0.1:8668"))]
    pub addr: Option<std::net::SocketAddr>,
}

impl Serve {
    pub async fn run(&self) {
        debug!("Serve: {self:?}");

        let meta = Meta::from_file(DEFAULT_METADATA_FILE).expect("Project meta.toml not found");
        debug!("Meta: {meta:?}");

        crate::server::start(self.addr.unwrap(), &meta)
            .instrument(debug_span!("[Http]"))
            .await
            .unwrap();
    }
}

/// Command Login
#[derive(Args, Debug)]
pub struct Login {
    /// The user_token
    pub user_token: String,
}

impl Login {
    pub async fn run(&self) {
        println!("Login: {self:?}");
    }
}

/// Command Deploy
#[derive(Args, Debug)]
pub struct Deploy {}

impl Deploy {
    pub async fn run(&self) {
        debug!("Deploy: {self:?}");

        let env_file = crate::env::get_metadata_env_file();
        debug!("Env file: {:?}", env_file);
        let env = match crate::env::CliEnv::from_file(&env_file) {
            Ok(env) => env,
            Err(e) => {
                debug!("Load env file failed: {:?}", e);
                error!("You are not logged. Run 'moni-cli login <your_token>'");
                std::process::exit(1);
            }
        };
        debug!("Env: {:?}", env);
    }
}
