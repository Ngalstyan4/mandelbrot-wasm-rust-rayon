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

// #[derive(Clone)]
struct FrameBuffer(std::cell::UnsafeCell<Vec<u8>>);

unsafe impl Sync for FrameBuffer {}

#[wasm_bindgen]
pub struct Scene {
    pub width: i32,
    pub height: i32,
    concurrency: usize,
    pool: std::sync::Arc<ThreadPool>,
    framebuffer: std::sync::Arc<FrameBuffer>,
    // framebuff: Uint8ClampedArray,
}

#[wasm_bindgen]
impl Scene {
    /// Creates a new scene from the JSON description in `object`, which we
    /// deserialize here into an actual scene.
    #[wasm_bindgen(constructor)]
    pub fn new(width: i32, height: i32, threads: usize, pool: &pool::WorkerPool) -> Result<Scene, JsValue> {
        console_error_panic_hook::set_once();

        // using vec! because the values needs to be heap allocated so it can be sent to another worker
        // and be reflected in *memory* of wasm ??i think?..??
        let mut nums = vec![0 as u8;(4* width * height) as usize];

        // Configure a rayon thread pool which will pull web workers from
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .spawn_handler(|thread| Ok(pool.run(|| thread.run()).unwrap()))
            .build()
            .unwrap();

        Ok(Scene {
            width, height, concurrency: threads,
            pool: std::sync::Arc::new(thread_pool),
            framebuffer: std::sync::Arc::new(FrameBuffer(std::cell::UnsafeCell::new(nums))),
        })
    }

    /// Renders this scene with the provided concurrency and worker pool.
    ///
    /// This will spawn up to `concurrency` workers which are loaded from or
    /// spawned into `pool`. The `RenderingScene` state contains information to
    /// get notifications when the render has completed.
    pub fn render(
        &mut self,
        pool: &pool::WorkerPool,
        scale: f64,
        dx: f64,
        dy: f64,
        num_iter: i32,
    ) -> Result<Promise, JsValue> {
        console_log!("in_render");
        // let nums = &self.framebuffer;
        // let mut nums = vec![0;5];
        let mut nums = self.framebuffer.clone();
        let thread_pool = self.pool.clone();
        let width = self.width as f64;
        let height = self.height as f64;


        unsafe {
            let (tx, rx) = oneshot::channel();
            pool.run(move || {
                thread_pool.install(|| {
                    (*nums.0.get())
                        .par_chunks_mut(4).enumerate().for_each(|(i, chunk)| {
                        let x = (i as f64 % width  - width/2.) / scale - dx;
                        let y = (i as f64 / width  - height/2.) / scale - dy;
                        let mut z = Complex { x: 0., y: 0. };
                        let cmlx = Complex { x, y };
                        let mut iter = 0;
                        for i in 0..num_iter {
                            iter += 1;
                            z = z * z + cmlx;
                            if z.magsq() > 4. { break }
                        }
                        if iter < num_iter {
                            let v = 255 - (255. * iter as f32 / num_iter as f32) as u8;
                            chunk[0] = v;
                            chunk[1] = v;
                            chunk[2] = v;
                        } else {
                            chunk[0] = 0;
                            chunk[1] = 0;
                            chunk[2] = 0;
                        }
                        // chunk[1] = 0;
                        // chunk[2] = 0;
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
                        let res = 42;//ImageData::new(&mem, width, height).unwrap();
                        Ok(res.into())
                    }
                    Err(_) => {
                        console_log!("errror");
                        Err(JsValue::undefined())
                    },
                }
            };

            Ok(wasm_bindgen_futures::future_to_promise(done))
        }
    }

    pub fn getBuffer(&self) -> ImageData {
        let mem = wasm_bindgen::memory().unchecked_into::<WebAssembly::Memory>();
        unsafe {
            let base = (*self.framebuffer.0.get()).as_ptr() as u32;
            let length = (self.width * self.height * 4) as u32;
            let mem = Uint8ClampedArray::new(&mem.buffer()).slice(base, (base + length));
            let res = ImageData::new(&mem, self.width as f64, self.height as f64).unwrap();
            res.into()
        }
    }

}

#[derive(Copy, Clone)]
struct Complex{
    x: f64,
    y: f64,
}

impl Complex {
    pub fn magsq(&self) -> f64 {(self.x * self.x + self.y * self.y)}
}

use std::ops::{Mul, Add};
use rayon::ThreadPool;
use wasm_bindgen::__rt::std::sync::Mutex;

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

// #[wasm_bindgen]
// pub struct RenderingScene {
//     promise: Promise,
// }
//
// #[wasm_bindgen]
// impl RenderingScene {
//     pub fn promise(&self) -> Promise {
//         self.promise.clone()
//     }
//
//
// }
//
#[wasm_bindgen]
extern "C" {
    pub type ImageData;

    #[wasm_bindgen(constructor,catch)]
    pub fn new(data: &Uint8ClampedArray, width: f64, height: f64) -> Result<ImageData, JsValue>;

}