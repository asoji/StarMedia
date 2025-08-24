use std::{
    ffi::c_void,
    sync::{
        Once, RwLock,
        mpsc::{Receiver, Sender, TryRecvError, channel},
    },
};

use jni::{
    JNIEnv,
    objects::{JClass, JObjectArray, JPrimitiveArray, JString},
};
use windows::{
    Foundation::TypedEventHandler,
    Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager, MediaPropertiesChangedEventArgs,
    },
    core::Interface,
};

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

#[unsafe(export_name = "Java_one_devos_nautical_starmedia_StarMediaLib_setPropertyChangedCallback")]
pub extern "system" fn properties_changed_callback<'local>(
    mut env: JNIEnv<'local>,
    _: JClass<'local>,
    gsmtcs: usize,
) -> JPrimitiveArray<'local, i64> {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let ptr = gsmtcs as *mut c_void;

        let current_session =
            unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };

        let handler = TypedEventHandler::new(properties_changed);

        throw_exception!(env, current_session.MediaPropertiesChanged(&handler));
    });

    let (tx, rx) = channel();

    let idx = {
        // TODO: This shold replace anything that is `None`
        let mut queues = throw_exception!(env, QUEUE.write());
        queues.push(Some(tx));
        queues.len() - 1
    };

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
    let rx = std::ptr::with_exposed_provenance_mut::<Receiver<SongInfo>>(ptr);
    let alloc = unsafe { Box::from_raw(rx) };
    drop(alloc);

    throw_exception!(env, QUEUE.write())[idx] = None;
}

static QUEUE: RwLock<Vec<Option<Sender<SongInfo>>>> = RwLock::new(Vec::new());

struct SongInfo {
    title: Box<str>,
    artist: Box<str>,
    subtitle: Box<str>,
    album_name: Box<str>,
    album_artist: Box<str>,
    album_len: i32,
    track_number: i32,
}

fn properties_changed(
    session: windows::core::Ref<'_, GlobalSystemMediaTransportControlsSession>,
    _: windows::core::Ref<'_, MediaPropertiesChangedEventArgs>,
) -> Result<(), windows::core::Error> {
    let session = session.unwrap();
    let props = session.TryGetMediaPropertiesAsync().unwrap().get().unwrap();

    let queues = &*QUEUE.read().unwrap();

    for queue in queues.iter().filter_map(Clone::clone) {
        let title = props.Title().unwrap().to_string().into_boxed_str();
        let artist = props.Artist().unwrap().to_string().into_boxed_str();
        let subtitle = props.Subtitle().unwrap().to_string().into_boxed_str();
        let album_name = props.AlbumTitle().unwrap().to_string().into_boxed_str();
        let album_artist = props.AlbumArtist().unwrap().to_string().into_boxed_str();
        let album_len = props.AlbumTrackCount().unwrap();
        let track_number = props.TrackNumber().unwrap();
        queue
            .send(SongInfo {
                title: title.clone(),
                artist: artist.clone(),
                subtitle: subtitle.clone(),
                album_name: album_name.clone(),
                album_artist: album_artist.clone(),
                album_len,
                track_number,
            })
            .unwrap();
    }

    Ok(())
}

#[unsafe(export_name = "Java_one_devos_nautical_starmedia_StarMediaLib_requestManager")]
pub extern "system" fn get_song_info<'local>(
    mut env: JNIEnv<'local>,
    _: JClass<'local>,
    ptr: usize,
) -> JObjectArray<'local> {
    let rx = std::ptr::with_exposed_provenance_mut::<Receiver<SongInfo>>(ptr);
    let rx = unsafe { &mut *rx };

    match rx.try_recv() {
        Ok(SongInfo {
            title,
            artist,
            subtitle,
            album_name,
            album_artist,
            album_len,
            track_number,
        }) => {
            let string_class = env.find_class("java/lang/String").unwrap();

            let array = env
                .new_object_array(7, string_class, JString::default())
                .unwrap();

            throw_exception!(
                env,
                env.set_object_array_element(
                    &array,
                    0,
                    throw_exception!(env, env.new_string(title))
                )
            );
            throw_exception!(
                env,
                env.set_object_array_element(
                    &array,
                    1,
                    throw_exception!(env, env.new_string(artist))
                )
            );
            throw_exception!(
                env,
                env.set_object_array_element(
                    &array,
                    2,
                    throw_exception!(env, env.new_string(subtitle))
                )
            );
            throw_exception!(
                env,
                env.set_object_array_element(
                    &array,
                    3,
                    throw_exception!(env, env.new_string(album_name))
                )
            );
            throw_exception!(
                env,
                env.set_object_array_element(
                    &array,
                    4,
                    throw_exception!(env, env.new_string(album_artist))
                )
            );
            throw_exception!(
                env,
                env.set_object_array_element(
                    &array,
                    5,
                    throw_exception!(env, env.new_string(album_len.to_string()))
                )
            );
            throw_exception!(
                env,
                env.set_object_array_element(
                    &array,
                    6,
                    throw_exception!(env, env.new_string(track_number.to_string()))
                )
            );

            array
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

    current_session.into_raw() as usize
}

#[unsafe(export_name = "Java_one_devos_nautical_starmedia_StarMediaLib_metadata")]
pub extern "system" fn metadata<'local>(
    mut env: JNIEnv<'local>,
    _: JClass<'local>,
    gsmtcs: usize,
) -> JObjectArray<'local> {
    let ptr = gsmtcs as *mut c_void;

    let current_session =
        unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };

    let props = throw_exception!(
        env,
        throw_exception!(env, current_session.TryGetMediaPropertiesAsync()).get()
    );

    let title = throw_exception!(env, props.Title()).to_string();
    let artist = throw_exception!(env, props.Artist()).to_string();
    let subtitle = throw_exception!(env, props.Subtitle()).to_string();
    let album_name = throw_exception!(env, props.AlbumTitle()).to_string();
    let album_artist = throw_exception!(env, props.AlbumArtist()).to_string();
    let album_len = throw_exception!(env, props.AlbumTrackCount()).to_string();
    let track_number = throw_exception!(env, props.TrackNumber()).to_string();
    let string_class = env.find_class("java/lang/String").unwrap();

    let array = env
        .new_object_array(7, string_class, JString::default())
        .unwrap();

    throw_exception!(
        env,
        env.set_object_array_element(&array, 0, throw_exception!(env, env.new_string(title)))
    );
    throw_exception!(
        env,
        env.set_object_array_element(&array, 1, throw_exception!(env, env.new_string(artist)))
    );
    throw_exception!(
        env,
        env.set_object_array_element(&array, 2, throw_exception!(env, env.new_string(subtitle)))
    );
    throw_exception!(
        env,
        env.set_object_array_element(&array, 3, throw_exception!(env, env.new_string(album_name)))
    );
    throw_exception!(
        env,
        env.set_object_array_element(
            &array,
            4,
            throw_exception!(env, env.new_string(album_artist))
        )
    );
    throw_exception!(
        env,
        env.set_object_array_element(&array, 5, throw_exception!(env, env.new_string(album_len)))
    );
    throw_exception!(
        env,
        env.set_object_array_element(
            &array,
            6,
            throw_exception!(env, env.new_string(track_number))
        )
    );

    array
}

#[unsafe(export_name = "Java_one_devos_nautical_starmedia_StarMediaLib_tryPause")]
pub extern "system" fn try_pause<'local>(
    mut env: JNIEnv<'local>,
    _: JClass<'local>,
    gsmtcs: usize,
) {
    let ptr = gsmtcs as *mut c_void;

    let gsmtcs =
        unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };

    throw_exception!(env, gsmtcs.TryPauseAsync());
}
