#![allow(renamed_and_removed_lints)]
#![allow(unknown_lints)]
#![allow(unused_unit)]

use std::error::Error;
use std::io::stdin;
use std::thread::{JoinHandle, spawn};

use bindings::component_sample::example::Adder;
use examples_common::get_wasm_module_path;
use hyperlight_wasm::Result as HyperlightResult;
use opentelemetry::trace::TracerProvider;
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_semantic_conventions::attribute::SERVICE_VERSION;
use tracing::{Level, span};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use uuid::Uuid;

extern crate alloc;
mod bindings {
    hyperlight_component_macro::host_bindgen!("../component_sample/wit/component-world.wasm");
}

const ENDPOINT_ADDR: &str = "http://localhost:4318/v1/traces";

fn init_tracing_subscriber(
    addr: &str,
) -> std::result::Result<SdkTracerProvider, Box<dyn Error + Send + Sync + 'static>> {
    let exporter = SpanExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_endpoint(addr)
        .build()?;

    let version = KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION"));
    let resource = Resource::builder()
        .with_service_name("hyperlight_wasm_otel_example")
        .with_attribute(version)
        .build();

    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build();

    global::set_tracer_provider(provider.clone());
    let tracer = provider.tracer("trace-demo");

    let otel_layer = OpenTelemetryLayer::new(tracer);

    // Try using the environment otherwise set default filters
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::from_default_env()
            .add_directive("hyperlight_host=info".parse().unwrap())
            .add_directive("tracing=info".parse().unwrap())
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(otel_layer)
        .try_init()?;

    Ok(provider)
}

pub struct State {}
impl State {
    pub fn new() -> Self {
        State {}
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl bindings::component_sample::example::Host for State {
    fn r#print(&mut self, message: alloc::string::String) {
        assert_eq!("42", message);
        println!("Logged from component: {message}");
    }

    fn r#host_function(&mut self, input: alloc::string::String) -> alloc::string::String {
        format!("{input} and the host!")
    }
}

#[allow(refining_impl_trait)]
impl bindings::component_sample::example::ExampleImports for State {
    type Host = State;

    fn r#host(&mut self) -> &mut Self {
        self
    }
}

fn example(wait_input: bool) -> HyperlightResult<()> {
    let mut join_handles: Vec<JoinHandle<HyperlightResult<()>>> = vec![];

    // Construct a new span named "hyperlight otel tracing example" with INFO  level.
    let span = span!(
        Level::INFO,
        "hyperlight-wasm otel component tracing example"
    );
    let _entered = span.enter();

    let handle = spawn(move || -> HyperlightResult<()> {
        // Construct a new span named "hyperlight tracing example thread" with INFO  level.
        let id = Uuid::new_v4();
        let span = span!(
            Level::INFO,
            "hyperlight tracing example thread",
            context = format!("Thread GUID {}", id),
            uuid = %id,
        );
        let _entered = span.enter();

        let state = State::new();
        let mut sb: hyperlight_wasm::ProtoWasmSandbox = builder()
            .with_guest_input_buffer_size(70 * 1024 * 1024)
            .with_guest_heap_size(200 * 1024 * 1024)
            .with_guest_stack_size(100 * 1024 * 1024)
            .build()
            .unwrap();
        let rt = bindings::register_host_functions(&mut sb, state);

        let sb = sb.load_runtime().unwrap();

        let mod_path = get_wasm_module_path("handler.aot").unwrap();
        println!("Loading Wasm Module from path: {:?}", mod_path);
        let sb = sb.load_module(mod_path).unwrap();

        let mut wrapped = bindings::ExampleSandbox { sb, rt };

        let instance = bindings::component_sample::example::ExampleExports::adder(&mut wrapped);
        let result = instance.add(1, 2);
        assert_eq!(3, result);
        println!("Add result is {result}");
        let result = instance.add(4, 3);
        assert_eq!(7, result);
        println!("Add result is {result}");
        instance.do_something(42);

        let result = instance.call_host("Hello".to_string());
        print!("Host Component interaction: {result}");
        Ok(())
    });
    join_handles.push(handle);

    if wait_input {
        println!("Press enter to exit...");
        let mut input = String::new();
        stdin().read_line(&mut input)?;
        // *should_exit.try_lock().unwrap() = true;
    }
    for join_handle in join_handles {
        let result = join_handle.join();
        assert!(result.is_ok());
    }

    Ok(())
}

fn builder() -> hyperlight_wasm::SandboxBuilder {
    #[cfg(gdb)]
    {
        hyperlight_wasm::SandboxBuilder::new().with_debugging_enabled(8080) // debugging on port 8080
    }
    #[cfg(not(gdb))]
    hyperlight_wasm::SandboxBuilder::new()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let provider = init_tracing_subscriber(ENDPOINT_ADDR)?;

    example(true)?;

    provider.shutdown()?;

    Ok(())
}
