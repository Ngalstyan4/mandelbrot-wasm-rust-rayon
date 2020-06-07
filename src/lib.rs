// use futures_channel::oneshot;
// use js_sys::{Promise, Uint8ClampedArray, WebAssembly};
use rayon::prelude::*;
use wasm_bindgen::prelude::*;
// use wasm_bindgen::JsCast;

macro_rules! console_log {
    ($($t:tt)*) => (crate::log(&format_args!($($t)*).to_string()))
}

mod pool;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn logv(x: &JsValue);
}

#[wasm_bindgen]
pub struct Scene {
    inner: i32
}

#[wasm_bindgen]
impl Scene {
    /// Creates a new scene from the JSON description in `object`, which we
    /// deserialize here into an actual scene.
    #[wasm_bindgen(constructor)]
    pub fn new(object: &JsValue) -> Result<Scene, JsValue> {
        console_error_panic_hook::set_once();
        Ok(Scene {
            inner: 42,
        })
    }

    /// Renders this scene with the provided concurrency and worker pool.
    ///
    /// This will spawn up to `concurrency` workers which are loaded from or
    /// spawned into `pool`. The `RenderingScene` state contains information to
    /// get notifications when the render has completed.
    pub fn render(
        self,
        concurrency: usize,
        pool: &pool::WorkerPool,
    ) {
        // let scene = self.inner;
        // let height = scene.height;
        // let width = scene.width;

        // Configure a rayon thread pool which will pull web workers from
        // `pool`.
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(concurrency)
            .spawn_handler(|thread| Ok(pool.run(|| thread.run()).unwrap()))
            .build()
            .unwrap();

        // thread_pool.install(|| {
        //
        // });

        // let done = async move {
        //     match rx.await {
        //         Ok(_data) => Ok(image_data(base, len, width, height).into()),
        //         Err(_) => Err(JsValue::undefined()),
        //     }
        // };
    }
}
