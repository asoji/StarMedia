use std::ops::Deref;
use std::ptr::with_exposed_provenance_mut;
use std::sync::{
    mpsc::{Receiver, Sender, TryRecvError},
    RwLock,
};

use crate::safe::{
    set_properties_changed_callback, try_extract_props, try_get_timeline_props, wrap_props_in_array,
};
use jni::objects::JLongArray;
use jni::sys::{jboolean, jint, jlongArray, jobjectArray};
use jni::{objects::{JClass, JObjectArray}, EnvUnowned};
use windows::{
    core::Interface, Media::Control::{
        GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    }
};

mod safe;

type PropsTuple = ([Box<str>; 5], [i32; 2], Box<[u8]>);

type SongInfoResult = Result<PropsTuple, windows::core::Error>;

static QUEUE: RwLock<Vec<Option<Sender<SongInfoResult>>>> = RwLock::new(Vec::new());

#[unsafe(export_name = "Java_gay_asoji_starmedia_StarMediaLib_setPropertyChangedCallback")]
pub extern "system" fn properties_changed_callback<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
    gsmtcs: usize,
) -> jlongArray {
    let outcome = unowned_env.with_env(|env| -> jni::errors::Result<jlongArray> {
        let ptr = with_exposed_provenance_mut(gsmtcs);

        let current_session =
            unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };
        let (idx, rx) = set_properties_changed_callback(current_session).unwrap();

        let array = env.new_long_array(2).unwrap();
        array.set_region(env, 0, &[
            idx as i64,
            Box::into_raw(Box::new(rx)).expose_provenance() as i64,
        ]).expect("Failed to set into region!");

        Ok(array.into_raw())
    });

    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(export_name = "Java_NativeGSMTC_dropReceiver")]
pub extern "system" fn drop_receiver_remove_sender<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
    ptr: usize,
    idx: usize,
) {
    let outcome = unowned_env.with_env(|env| -> jni::errors::Result<_> {
        let rx = with_exposed_provenance_mut::<Receiver<SongInfoResult>>(ptr);
        let alloc = unsafe { Box::from_raw(rx) };
        drop(alloc);

        QUEUE.write().unwrap()[idx] = None;
        Ok(())
    });

    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}

#[unsafe(export_name = "Java_gay_asoji_starmedia_StarMediaLib_getSongInfo")]
pub extern "system" fn get_song_info<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
    ptr: usize,
) -> jobjectArray {
    let outcome = unowned_env.with_env(|env| -> jni::errors::Result<jobjectArray> {
        let rx = with_exposed_provenance_mut::<Receiver<SongInfoResult>>(ptr);
        let rx = unsafe { &mut *rx };

        match rx.try_recv() {
            Ok(result) => {
                Ok(wrap_props_in_array(env, result.unwrap()).into_raw())
            }
            Err(TryRecvError::Empty) => Ok(jobjectArray::default()),
            Err(TryRecvError::Disconnected) => {
                panic!("Receiver was disconnected")
            }
        }
    });

    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(export_name = "Java_gay_asoji_starmedia_StarMediaLib_requestManager")]
pub extern "system" fn request_manager<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
) -> usize {
    let outcome = unowned_env.with_env(|_env| -> jni::errors::Result<usize> {
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync().unwrap();
        let gsmtcs = manager.join().unwrap();

        Ok(gsmtcs.into_raw().expose_provenance())
    });

    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(export_name = "Java_gay_asoji_starmedia_StarMediaLib_requestSession")]
pub extern "system" fn request_session<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
    manager: usize,
) -> usize {
    let outcome = unowned_env.with_env(|_env| -> jni::errors::Result<usize> {
        let ptr = with_exposed_provenance_mut(manager);
        let gsmtcs = unsafe { GlobalSystemMediaTransportControlsSessionManager::from_raw_borrowed(&ptr).unwrap() };
        let current_session = gsmtcs.GetCurrentSession().unwrap();

        Ok(current_session.into_raw().expose_provenance())
    });

    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(export_name = "Java_gay_asoji_starmedia_StarMediaLib_metadata")]
pub extern "system" fn metadata<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
    gsmtcs: usize,
) -> JObjectArray<'local> {
    let outcome = unowned_env.with_env(|env| -> jni::errors::Result<JObjectArray<'local>> {
        let ptr = with_exposed_provenance_mut(gsmtcs);

        let current_session =
            unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };

        let info = try_extract_props(current_session).unwrap();
        Ok(wrap_props_in_array(env, info))
    });

    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(export_name = "Java_gay_asoji_starmedia_StarMediaLib_timeline")]
pub extern "system" fn timeline<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
    gsmtcs: usize,
) -> JLongArray<'local> {
    let outcome = unowned_env.with_env(|env| -> jni::errors::Result<JLongArray> {
        let ptr = with_exposed_provenance_mut(gsmtcs);
        let current_session =
            unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };

        let timeline_props = try_get_timeline_props(current_session).unwrap();

        let array = env.new_long_array(timeline_props.len()).unwrap();
        array.set_region(env, 0, &timeline_props).unwrap();

        Ok(array)
    });

    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(export_name = "Java_gay_asoji_starmedia_StarMediaLib_tryPause")]
pub extern "system" fn try_pause<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
    gsmtcs: usize,
) -> jboolean {
    let outcome = unowned_env.with_env(|_env| -> jni::errors::Result<jboolean> {
        let ptr = with_exposed_provenance_mut(gsmtcs);

        let gsmtcs =
            unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };

        Ok(gsmtcs.TryPauseAsync().unwrap().join().unwrap())
    });

    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(export_name = "Java_gay_asoji_starmedia_StarMediaLib_tryPlay")]
pub extern "system" fn try_play<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
    gsmtcs: usize,
) -> bool {
    let outcome = unowned_env.with_env(|_env| -> jni::errors::Result<jboolean> {
        let ptr = with_exposed_provenance_mut(gsmtcs);

        let gsmtcs =
            unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };

        Ok(gsmtcs.TryPlayAsync().unwrap().join().unwrap())
    });

    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(export_name = "Java_gay_asoji_starmedia_StarMediaLib_tryTogglePlayPause")]
pub extern "system" fn try_toggle_pause_play<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
    gsmtcs: usize,
) -> bool {
    let outcome = unowned_env.with_env(|_env| -> jni::errors::Result<jboolean> {
        let ptr = with_exposed_provenance_mut(gsmtcs);

        let gsmtcs =
            unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };

        Ok(gsmtcs.TryTogglePlayPauseAsync().unwrap().join().unwrap())
    });

    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(export_name = "Java_gay_asoji_starmedia_StarMediaLib_trySkipNext")]
pub extern "system" fn try_skip_next<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
    gsmtcs: usize,
) -> bool {
    let outcome = unowned_env.with_env(|_env| -> jni::errors::Result<jboolean> {
        let ptr = with_exposed_provenance_mut(gsmtcs);

        let gsmtcs =
            unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };

        Ok(gsmtcs.TrySkipNextAsync().unwrap().join().unwrap())
    });

    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(export_name = "Java_gay_asoji_starmedia_StarMediaLib_trySkipPrevious")]
pub extern "system" fn try_skip_previous<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
    gsmtcs: usize,
) -> bool {
    let outcome = unowned_env.with_env(|_env| -> jni::errors::Result<jboolean> {
        let ptr = with_exposed_provenance_mut(gsmtcs);

        let gsmtcs =
            unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };

        Ok(gsmtcs.TrySkipPreviousAsync().unwrap().join().unwrap())
    });

    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(export_name = "Java_gay_asoji_starmedia_StarMediaLib_getStatus")]
pub extern "system" fn get_status<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
    gsmtcs: usize,
) -> jint {
    let outcome = unowned_env.with_env(|_env| -> jni::errors::Result<jint> {
        let ptr = with_exposed_provenance_mut(gsmtcs);

        let gsmtcs =
            unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };

        Ok(gsmtcs.GetPlaybackInfo().unwrap().PlaybackStatus().unwrap().0)
    });

    outcome.resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

