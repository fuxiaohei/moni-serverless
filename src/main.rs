use clap::Parser;
use moni_core::keyvalue::SledStorage;
use moni_runtime::Context;
use std::sync::Arc;
use tokio::sync::Mutex;

/// get_default_kv_db returns default kv db path
pub fn get_default_kv_db() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home)
        .join(".moni_keyvalue_db")
        .to_str()
        .unwrap()
        .to_string()
}

#[derive(Parser, Debug)]
struct CliArgs {
    name: String,
    #[clap(long)]
    url: Option<String>,
}

#[tokio::main]
async fn main() {
    let start_time = tokio::time::Instant::now();

    let args = CliArgs::parse();

    let name = args.name.replace('-', "_");
    println!("Run name\t: {name}");

    let arch = "wasm32-wasi";
    println!("Run arch\t: {arch}");

    let target = format!("target/{arch}/release/{name}.wasm");
    let output = format!("target/{arch}/release/{name}.component.wasm");

    moni_runtime::convert_component(&target, Some(output.to_string())).unwrap();
    println!("Run component\t: {output}");

    let worker_pool = moni_runtime::create_pool(&output).unwrap();
    let status = worker_pool.status();
    println!("Pool status\t, {status:?}");

    let mut worker = worker_pool.get().await.unwrap();
    let worker = worker.as_mut();

    let headers = vec![("Content-Type", "application/json")];
    let url = args.url.unwrap_or("/abc".to_string());
    let req = moni_runtime::http_impl::http_handler::Request {
        method: "GET",
        uri: url.as_str(),
        headers: &headers,
        body: None,
    };

    let kvdb = SledStorage::new(&get_default_kv_db()).unwrap();
    let provider = Arc::new(Mutex::new(kvdb));
    let mut context = Context::new();
    context.set_kv_provider(provider);

    let resp = worker.handle_request(req, context).await.unwrap();
    println!("-----\nstatus, {:?}", resp.status);
    for (key, value) in resp.headers {
        println!("header\t, {key}: {value}");
    }

    let body_length = resp.body.as_ref().unwrap().len();
    println!("body_length\t, {body_length:?}");

    if body_length < 128 {
        println!(
            "body_short_cnt\t, {:?}",
            String::from_utf8(resp.body.unwrap()).unwrap()
        );
    }
    println!("elapsed\t, {:?}", start_time.elapsed());

    if resp.status >= 400 {
        panic!("error status: {:?}", resp.status)
    }
    println!("-----");
}
