
use std::{
    ops::{Deref,DerefMut}
};

use wasm_bindgen::{
    prelude::*,
    convert::FromWasmAbi,
    closure::Closure, JsCast, JsValue,
};

use js_sys::Function;

use wasm_bindgen_futures::{spawn_local, JsFuture};

use web_sys::{
    AudioContext,
    AudioProcessingEvent, 
    MediaStream, 
    MediaStreamConstraints
};

use audio_recorder::{
    math,
    codec::{AdhocCodec},
    collections::{ LinkedList, Ptr},
    signal
};

pub  static mut GLOBAL_APP_STATE:Option<AppState> = None; 




pub struct JsCallBackPool {
    handlers: LinkedList<Option<JsCallbackHandler>>,
}
impl JsCallBackPool {
    pub fn new() -> Self {
        Self {
            handlers: LinkedList::new(),
        }
    }

    pub fn register_handler(&mut self, callback: js_sys::Function) -> JsCallbackHandler {
        self.handlers.push_rear(None);
        let thread_id = self.handlers.rear();

        let handler = JsCallbackHandler {
            thread_id,
            callback,
        };

        self.handlers
            .get_mut(thread_id)
            .map(|node| node.set_data(Some(handler.clone())));

        handler
    }
}

#[derive(Clone)]
pub struct JsCallbackHandler {
    pub thread_id: Ptr,
    pub callback: js_sys::Function,
}

pub static mut LMAO_A_GLOBAL_FUNCTION: Option<js_sys::Function> = None;



pub struct AppState{
    output_buffer:Vec<f32>,
    audio_codec:AdhocCodec,
    callback_registry: JsCallBackPool,
}
impl AppState{
    fn init(){
        unsafe{
            GLOBAL_APP_STATE = Some(
                AppState{
                    output_buffer:vec![0.0;1024],
                    audio_codec: AdhocCodec::new(),
                    callback_registry: JsCallBackPool::new(),
                }
            );
        }
    }
    fn get()->&'static Self{
        unsafe{
            GLOBAL_APP_STATE.as_ref().unwrap()
        }
    }

    fn get_mut()->&'static mut Self{
        unsafe{
            GLOBAL_APP_STATE.as_mut().unwrap()
        }
    }
}








#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace=console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace=console,js_name=log)]
    fn log_js(val: JsValue);

    #[wasm_bindgen(js_namespace=console,js_name=log)]
    fn log_u32(val: u32);
}

pub fn closure_to_function<CB, T>(cb: CB) -> js_sys::Function
where
    T: FromWasmAbi + 'static,
    CB: FnMut(T) + 'static,
{
    Closure::wrap(Box::new(cb) as Box<dyn FnMut(T)>)
        .into_js_value().dyn_into::<Function>()
        .unwrap()
}





#[test]
fn gauss_test(){
    let samples =signal::gaussian_filter::<7>(1.0,5.0);
    println!("{:?}",samples);
}



async fn start() -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let navigator = window.navigator();

    let mut constraints = MediaStreamConstraints::new();
    let ctx: AudioContext = AudioContext::new()?;


    AppState::init();
    let mut max_ratio = 0.0f32;  

    let handler = AppState::get_mut().callback_registry.register_handler(closure_to_function( move |e: AudioProcessingEvent| {
        let micophone_input = e.input_buffer().unwrap();
        let speaker_output = e.output_buffer().unwrap();   
        
    

        let microphone_samples = micophone_input.get_channel_data(0).unwrap_or(Vec::new());


        // let kernel = signal::gaussian_filter::<9>(1.5, 2.0);
        let mut smoothed_input = [0f32;1024];
        // signal::convolve_1d_branchless(&microphone_samples, &kernel,&mut smoothed_input );
        // let edge_count =16;
        // for k in 0..edge_count{
        //     let mut t = ((k as f32)/(edge_count-1) as f32).min(1.0); 
        //     t*=t;
        //     smoothed_input[k] = microphone_samples[k]*(1.0-t) + smoothed_input[k]*t;
        // }
        // for k in (microphone_samples.len()-edge_count)..microphone_samples.len() {
        //     let mut t = ((k as f32)/(edge_count-1) as f32).min(1.0); 
        //     t*=t;
        //     smoothed_input[k] = microphone_samples[k]*(t) + smoothed_input[k]*(1.0-t);
        // }

        // log(format!("buffer length = {}",in_samps.len()).as_str());



        // let audio_codec = &mut AppState::get_mut().audio_codec;
        // audio_codec.init();


        

        
        // let scaled_signal = signal::scale_signal(&microphone_samples,dst_len,&mut voice_upsampled);
        let mut decoded_signal = [0.0;1024];

        // let mut codec = AudioStreamCodec::new();
        // codec.init();
        // codec.encode(&microphone_samples);

        // codec.init();
        // codec.decode(&mut decoded_signal[..]);
        // let mean_squared_error = math::compute_mse(&microphone_samples, &decoded_signal );


        // let compression_ratio = codec.stream.stream.capacity() as f32 / (smoothed_input.len() * 16) as f32; 
        // max_ratio = max_ratio.max(compression_ratio);
        // let log_str = format!(
        //     "blocks allocated = {}\nbits written = {}\ncompression_ratio={:.4}\nmax_ratio={:.4}\nMean Squared Error={:.6}",
        //     codec.stream.stream.binary.len(),
        //     codec.stream.stream.capacity(),
        //     compression_ratio,
        //     max_ratio,
        //     mean_squared_error
        // );
        // log(&log_str);

        speaker_output.copy_to_channel(&decoded_signal[..], 0).unwrap();
    }));



    JsFuture::from(
        navigator
            .media_devices()?
            .get_user_media_with_constraints(
                &constraints
                    .audio(&JsValue::from_bool(true))
                    .video(&JsValue::from_bool(false)),
            )?
            .then(&Closure::wrap(Box::new(move |stream: JsValue| {
                log("entered stream");
                
                let stream = stream.dyn_into::<MediaStream>().unwrap();
                
                let source = ctx.create_media_stream_source(&stream).unwrap();
                
                let processor = ctx.create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(1024, 1, 1).unwrap();
                
                source.connect_with_audio_node( processor.dyn_ref().unwrap()  ).unwrap();
                
                let audioprocess_cb = AppState::get()
                    .callback_registry
                    .handlers[handler.thread_id]
                    .data()
                    .unwrap()
                    .as_ref()
                    .map(|e| &e.callback);

                processor.set_onaudioprocess(audioprocess_cb);
                
                processor.connect_with_audio_node(ctx.destination().dyn_ref().unwrap()).unwrap();

            }) as Box<dyn FnMut(_)>)),
    )
    .await?;

    // JsFuture::from(
    //     ctx.audio_worklet()?
    //         .add_module("white_noise_processor.js")?,
    // )
    // .await?;
    // let white_noise_node = AudioWorkletNode::new(&ctx,"white_noise_processor")?;
    // white_noise_node.connect_with_audio_node(&ctx.destination())?;

    Ok(())
}

fn main() {
    spawn_local(async {
        match start().await {
            Ok(_) => {
                log("module exited nicely");
            }
            Err(_) => {
                log("we hit an error somewhere... ");
            }
        }
    });
}
