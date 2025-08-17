use std::{
    ffi::c_void,
    ptr::null_mut,
    sync::{OnceLock, atomic::AtomicPtr},
};

use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JClass, JObject, JObjectArray, JString, JValue},
    signature::{JavaType, TypeSignature},
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

#[unsafe(export_name = "Java_NativeGSMTC_setPropertyChangedCallback")]
pub extern "system" fn properties_changed_callback<'local>(
    mut env: JNIEnv<'local>,
    _: JClass<'local>,
    callback: JObject<'local>,
    gsmtcs: usize,
) {
    let ptr = gsmtcs as *mut c_void;

    let current_session =
        unsafe { GlobalSystemMediaTransportControlsSession::from_raw_borrowed(&ptr).unwrap() };

    let jvm = throw_exception!(env, env.get_java_vm());
    JVM.get_or_init(|| jvm);

    let callback = env.new_global_ref(callback).unwrap();
    CALLBACK.get_or_init(|| callback);

    let handler = TypedEventHandler::new(properties_changed);

    throw_exception!(env, current_session.MediaPropertiesChanged(&handler));

    let raw = handler.into_raw();

    RAW.store(raw, std::sync::atomic::Ordering::SeqCst);
}

static JVM: OnceLock<JavaVM> = OnceLock::new();
static CALLBACK: OnceLock<GlobalRef> = OnceLock::new();
static RAW: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

// TODO: Handle exceptions in this callback, see:
// https://github.com/jni-rs/jni-rs/issues/533#issuecomment-2087863105
fn properties_changed(
    session: windows::core::Ref<'_, GlobalSystemMediaTransportControlsSession>,
    _: windows::core::Ref<'_, MediaPropertiesChangedEventArgs>,
) -> Result<(), windows::core::Error> {
    let jvm = JVM.get().unwrap();

    {
        let mut guard = jvm.attach_current_thread().unwrap();
        let callback = CALLBACK.get().unwrap();

        let string_arg = jni::signature::JavaType::Object("java/lang/String".into());

        let sig = TypeSignature {
            args: vec![
                string_arg.clone(),
                string_arg.clone(),
                string_arg.clone(),
                string_arg.clone(),
                string_arg,
                JavaType::Primitive(jni::signature::Primitive::Int),
                JavaType::Primitive(jni::signature::Primitive::Int),
            ],
            ret: jni::signature::ReturnType::Primitive(jni::signature::Primitive::Void),
        };

        let session = session.unwrap();
        let props = session.TryGetMediaPropertiesAsync().unwrap().get().unwrap();

        let title = props.Title().unwrap().to_string();
        let artist = props.Artist().unwrap().to_string();
        let subtitle = props.Subtitle().unwrap().to_string();
        let album_name = props.AlbumTitle().unwrap().to_string();
        let album_artist = props.AlbumArtist().unwrap().to_string();
        let album_len = props.AlbumTrackCount().unwrap();
        let track_number = props.TrackNumber().unwrap();

        let jtitle = guard.new_string(title).unwrap();
        let jartist = guard.new_string(artist).unwrap();
        let jsubtitle = guard.new_string(subtitle).unwrap();
        let jalbum_name = guard.new_string(album_name).unwrap();
        let jalbum_artist = guard.new_string(album_artist).unwrap();

        guard
            .call_method(
                callback,
                "propertiesChanged",
                sig.to_string(),
                &[
                    JValue::Object(&jtitle),
                    JValue::Object(&jartist),
                    JValue::Object(&jsubtitle),
                    JValue::Object(&jalbum_name),
                    JValue::Object(&jalbum_artist),
                    JValue::Int(album_len),
                    JValue::Int(track_number),
                ],
            )
            .unwrap();
    }

    Ok(())
}

#[unsafe(export_name = "Java_NativeGSMTC_requestManager")]
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

#[unsafe(export_name = "Java_NativeGSMTC_metadata")]
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
        env.set_object_array_element(&array, 1, throw_exception!(env, env.new_string(subtitle)))
    );
    throw_exception!(
        env,
        env.set_object_array_element(&array, 2, throw_exception!(env, env.new_string(artist)))
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

#[unsafe(export_name = "Java_NativeGSMTC_tryPause")]
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
