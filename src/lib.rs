use futures_channel::oneshot;
use js_sys::{Promise, Uint8ClampedArray, WebAssembly};
use rayon::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;// needed for unchecked_into

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
    pub fn new(_object: &JsValue) -> Result<Scene, JsValue> {
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
    ) -> Result<RenderingScene, JsValue> {
        console_log!("in_render");

        // using vec! because the values needs to be heap allocated so it can be sent to another worker
        // and be reflected in *memory* of wasm ??i think?..??
        let mut nums = vec![0 as u8;4*300*400];

        // Configure a rayon thread pool which will pull web workers from
        // `pool`.
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(concurrency)
            .spawn_handler(|thread| Ok(pool.run(|| thread.run()).unwrap()))
            .build()
            .unwrap();


        let (tx, rx) = oneshot::channel();
        pool.run(move || {
            thread_pool.install(|| {
                nums.par_chunks_mut(4).enumerate().for_each(|(i,chunk )| {
                    let scale = 100.;
                    let x = i as f32 % 300. / scale - 1.8;
                    let y = i as f32 / 300. / scale - 0.8;
                    let mut z = Complex{x:0.,y:0.};
                    let cmlx = Complex{x,y};
                    let NUM_ITER = 100000;
                    let mut escaped = false;
                    for i in 0..NUM_ITER {
                        z = z * z + cmlx;
                        if z.magsq() > 4. {escaped = true; break}
                    }
                    if escaped{
                        chunk[0] = 255;
                    } else {
                        chunk[0] = 0;
                    }
                    chunk[1] = 0;
                    chunk[2] = 0;
                    chunk[3] = 255;

                });
            });
            drop(tx.send(nums));

        })?;
        console_log!("waiting for done");

        let done = async move {
            console_log!("done!");
            match rx.await {

                Ok(_data) => {
                    // console_log!("got data  {}", _data);
                    let mem = wasm_bindgen::memory().unchecked_into::<WebAssembly::Memory>();
                    console_log!("data0 {} data1 {} dataLen{}", _data[0], _data[1], _data.len());
                    let mem = Uint8ClampedArray::new(&mem.buffer()).slice(_data.as_ptr() as u32, _data.as_ptr() as u32 + _data.len() as u32);
                    let res = ImageData::new(&mem, 300.,400.).unwrap();
                    Ok(res.into())
                }
                Err(_) =>  {console_log!("errror") ;Err(JsValue::undefined())},
            }
        };

        Ok(RenderingScene{promise: wasm_bindgen_futures::future_to_promise(done)})

    }

}

#[derive(Copy, Clone)]
struct Complex{
    x: f32,
    y: f32,
}

impl Complex {
    pub fn magsq(&self) -> f32 {(self.x * self.x + self.y * self.y)}
}

use std::ops::{Mul, Add};

impl Mul for Complex {
    type Output = Complex;

    fn mul(self, rhs: Self) -> Self::Output {
        Complex{
            x: self.x * rhs.x - self.y * rhs.y,
            y: self.x * rhs.y + self.y * rhs.x}
    }
}

impl Add for Complex {
    type Output = Complex;

    fn add(self, rhs: Self) -> Self::Output {
        Complex {
            x:self.x+rhs.x, y: self.y+rhs.y
        }
    }
}

#[wasm_bindgen]
pub struct RenderingScene {
    promise: Promise,
}

#[wasm_bindgen]
impl RenderingScene {
    pub fn promise(&self) -> Promise {
        self.promise.clone()
    }
}
//
#[wasm_bindgen]
extern "C" {
    pub type ImageData;

    #[wasm_bindgen(constructor,catch)]
    pub fn new(data: &Uint8ClampedArray, width: f64, height: f64) -> Result<ImageData, JsValue>;

}
//
// fn create_compute() -> ComputeResult {
//     ComputeResult::new()
// }
