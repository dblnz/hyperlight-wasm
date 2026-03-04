#![allow(renamed_and_removed_lints)]
#![allow(unknown_lints)]
#![allow(unused_unit)]

use std::error::Error;
use std::io::stdin;
use std::sync::{Arc, Mutex};
use std::thread::{JoinHandle, spawn};

use examples_common::get_wasm_module_path;
use hyperlight_host::Result as HyperlightResult;
use opentelemetry::trace::TracerProvider;
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_semantic_conventions::attribute::SERVICE_VERSION;
use tracing::{Level, span};
use tracing_flame::FlameLayer;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::Registry;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};
use uuid::Uuid;

extern crate alloc;
mod bindings {
    hyperlight_component_macro::host_bindgen!("../js_component_sample/wit/component-world.wasm");
}

use bindings::hyperlight::bench::HandlerInterface;
use bindings::hyperlight::bench::handler_interface::Request;

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

impl bindings::hyperlight::bench::HostInterface for State {
    fn r#print(&mut self, message: alloc::string::String) {
        println!("Logged from component: {message}");
    }
}

#[allow(refining_impl_trait)]
impl bindings::hyperlight::bench::HandlerWorldImports for State {
    type HostInterface = State;

    fn r#host_interface(&mut self) -> &mut Self {
        self
    }
}

#[cfg(feature = "otel")]
const ENDPOINT_ADDR: &str = "http://localhost:4318/v1/traces";

#[cfg(feature = "otel")]
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

// fn setup_global_subscriber() -> impl Drop {
//     let fmt_layer = fmt::Layer::default();
//
//     let (flame_layer, _guard) = FlameLayer::with_file("./tracing.folded").unwrap();
//
//     let subscriber = Registry::default().with(fmt_layer).with(flame_layer);
//
//     tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
//     _guard
// }

#[cfg(feature = "tracy")]
fn setup_global_subscriber() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::default()),
    )
    .expect("setup tracy layer");
}

fn main() {
    #[cfg(feature = "otel")]
    let provider = init_tracing_subscriber(ENDPOINT_ADDR).unwrap();
    let wait_input = true;
    #[cfg(feature = "tracy")]
    let _guard = setup_global_subscriber();

    let mut join_handles: Vec<JoinHandle<HyperlightResult<()>>> = vec![];

    // Construct a new span named "hyperlight otel tracing example" with INFO  level.
    let span = span!(Level::INFO, "hyperlight otel tracing example");
    let _entered = span.enter();

    let components = vec![
        (
            get_wasm_module_path("component_sample.aot").unwrap(),
            1 * 1024 * 1024,
        ),
        (
            get_wasm_module_path("js_component_sample.aot").unwrap(),
            200 * 1024 * 1024,
        ),
    ];

    for (component, size) in components {
        // Construct a new span named "hyperlight tracing example thread" with INFO  level.
        let id = Uuid::new_v4();
        let span = span!(
            Level::INFO,
            "hyperlight tracing example thread",
            context = format!("Component {} GUID {}", component.clone(), id),
            uuid = %id,
        );
        let _entered = span.enter();

        let state = State::new();
        let mut sb: hyperlight_wasm::ProtoWasmSandbox = hyperlight_wasm::SandboxBuilder::new()
            .with_guest_input_buffer_size(10 * 1024)
            .with_guest_heap_size(size)
            .with_guest_stack_size(1 * 1024 * 1024)
            .with_guest_scratch_size(70 * 1024 * 1024)
            //.with_debugging_enabled(8080)
            .build()
            .unwrap();
        let rt = bindings::register_host_functions(&mut sb, state);

        let sb = sb.load_runtime().unwrap();

        let mod_path = component.clone();
        let sb = sb.load_module(mod_path).unwrap();

        let mut wrapped = bindings::HandlerWorldSandbox { sb, rt };

        let instance =
            bindings::hyperlight::bench::HandlerWorldExports::handler_interface(&mut wrapped);
        let request = Request {
            uri: "Test Event".to_string(),
        };
        let span2 = span!(Level::INFO, "calling handleevent",);
        let _entered2 = span2.enter();
        let result: Request = instance.handleevent(request);
        drop(_entered2);
        println!("Result is: {:?}", result);
        let span3 = span!(Level::INFO, "calling fib",);
        let _entered3 = span3.enter();
        let result = instance.fib(10i32);
        drop(_entered3);
        println!("Fib result is: {:?}", result);
        drop(_entered);
    }

    if wait_input {
        println!("Press enter to exit...");
        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
    }

    for join_handle in join_handles {
        let result = join_handle.join();
        assert!(result.is_ok());
    }
    #[cfg(feature = "otel")]
    provider.shutdown();
}
