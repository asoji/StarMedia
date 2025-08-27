use std::sync::{
    RwLock,
    mpsc::{Receiver, Sender, TryRecvError},
};

use jni::{
    JNIEnv,
    objects::{JClass, JObjectArray, JPrimitiveArray},
};
use windows::{
    Media::Control::{
        GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    },
    core::Interface,
};

use crate::safe::{set_properties_changed_callback, try_extract_props, wrap_props_in_array};

mod safe;

type PropsTuple = ([Box<str>; 5], [i32; 2], Box<[u8]>);

type SongInfoResult = Result<PropsTuple, windows::core::Error>;

static QUEUE: RwLock<Vec<Option<Sender<SongInfoResult>>>> = RwLock::new(Vec::new());

#[unsafe(export_name = "Java_one_devos_nautical_starmedia_StarMediaLib_setPropertyChangedCallback")]
pub extern "system" fn properties_changed_callback<'local>(
    mut env: JNIEnv<'local>,
    _: JClass<'local>,
    gsmtcs: usize,
) -> JPrimitiveArray<'local, i64> {
    let ptr = std::ptr::with_exposed_provenance_mut(gsmtcs);

    let current_session =
        unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };
    let (idx, rx) = throw_exception!(env, set_properties_changed_callback(current_session));

    let array = throw_exception!(env, env.new_long_array(2));
    throw_exception!(
        env,
        env.set_long_array_region(
            &array,
            0,
            &[
                idx as i64,
                Box::into_raw(Box::new(rx)).expose_provenance() as i64,
            ],
        )
    );

    array
}

#[unsafe(export_name = "Java_NativeGSMTC_dropReciever")]
pub extern "system" fn drop_reciever_remove_sender<'local>(
    mut env: JNIEnv<'local>,
    _: JClass<'local>,
    ptr: usize,
    idx: usize,
) {
    let rx = std::ptr::with_exposed_provenance_mut::<Receiver<SongInfoResult>>(ptr);
    let alloc = unsafe { Box::from_raw(rx) };
    drop(alloc);

    throw_exception!(env, QUEUE.write())[idx] = None;
}

#[unsafe(export_name = "Java_one_devos_nautical_starmedia_StarMediaLib_requestManager")]
pub extern "system" fn get_song_info<'local>(
    mut env: JNIEnv<'local>,
    _: JClass<'local>,
    ptr: usize,
) -> JObjectArray<'local> {
    let rx = std::ptr::with_exposed_provenance_mut::<Receiver<SongInfoResult>>(ptr);
    let rx = unsafe { &mut *rx };

    match rx.try_recv() {
        Ok(result) => {
            let info = throw_exception!(env, result);
            wrap_props_in_array(env, info)
        }
        Err(TryRecvError::Empty) => JObjectArray::default(),
        Err(TryRecvError::Disconnected) => {
            throw_exception!(env, Err("Reciever was disconnected"))
        }
    }
}

#[unsafe(export_name = "Java_one_devos_nautical_starmedia_StarMediaLib_requestManager")]
pub extern "system" fn request_manager<'local>(
    mut env: JNIEnv<'local>,
    _: JClass<'local>,
) -> usize {
    let manager = throw_exception!(
        env,
        GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
    );

    let gsmtcs = throw_exception!(env, manager.get());

    let current_session = throw_exception!(env, gsmtcs.GetCurrentSession());

    current_session.into_raw().expose_provenance()
}

#[unsafe(export_name = "Java_one_devos_nautical_starmedia_StarMediaLib_metadata")]
pub extern "system" fn metadata<'local>(
    mut env: JNIEnv<'local>,
    _: JClass<'local>,
    gsmtcs: usize,
) -> JObjectArray<'local> {
    let ptr = std::ptr::with_exposed_provenance_mut(gsmtcs);

    let current_session =
        unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };

    let info = throw_exception!(env, try_extract_props(current_session));
    wrap_props_in_array(env, info)
}

#[unsafe(export_name = "Java_one_devos_nautical_starmedia_StarMediaLib_tryPause")]
pub extern "system" fn try_pause<'local>(
    mut env: JNIEnv<'local>,
    _: JClass<'local>,
    gsmtcs: usize,
) {
    let ptr = std::ptr::with_exposed_provenance_mut(gsmtcs);

    let gsmtcs =
        unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };

    throw_exception!(env, gsmtcs.TryPauseAsync());
}

#[macro_export]
macro_rules! throw_exception {
    ($env:ident, $try:expr) => {
        match $try {
            Ok(ok) => ok,
            Err(e) => {
                if !$env.exception_check().unwrap() {
                    $env.throw(e.to_string()).unwrap();
                }
                return Default::default();
            }
        }
    };
}
