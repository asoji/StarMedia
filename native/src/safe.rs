use std::{
    error::Error,
    sync::{
        atomic::AtomicBool,
        mpsc::{Receiver, channel},
    },
};

use jni::{
    JNIEnv,
    objects::{JObject, JObjectArray, JValueGen},
};
use windows::{
    Foundation::TypedEventHandler,
    Media::Control::{GlobalSystemMediaTransportControlsSession, MediaPropertiesChangedEventArgs},
    Storage::Streams::{Buffer, DataReader, InputStreamOptions},
};

use crate::{PropsTuple, QUEUE, SongInfoResult, throw_exception};

pub fn properties_changed(
    session: windows::core::Ref<'_, GlobalSystemMediaTransportControlsSession>,
    _: windows::core::Ref<'_, MediaPropertiesChangedEventArgs>,
) -> Result<(), windows::core::Error> {
    let session = session.unwrap();
    let queues = &*QUEUE.read().unwrap();

    for queue in queues.iter().filter_map(Clone::clone) {
        let result = try_extract_props(session);
        queue.send(result).unwrap();
    }

    Ok(())
}

pub fn wrap_props_in_array<'local>(
    mut env: JNIEnv<'local>,
    info: PropsTuple,
) -> JObjectArray<'local> {
    let (strings, ints, buf) = info;

    let obj_class = env.find_class("java/lang/Object").unwrap();
    let byte_buffer = env.find_class("java/nio/ByteBuffer").unwrap();
    let integer = env.find_class("java/lang/Integer").unwrap();
    let buf_wrapper = env.byte_array_from_slice(&buf).unwrap();

    let array = env
        .new_object_array(8, obj_class, JObject::default())
        .unwrap();

    for (idx, string) in strings.iter().enumerate() {
        let jstring = throw_exception!(env, env.new_string(string));
        throw_exception!(
            env,
            env.set_object_array_element(&array, idx as i32, jstring)
        );
    }

    for (idx, int) in ints.iter().enumerate() {
        let jint = throw_exception!(
            env,
            env.new_object(&integer, "I)Ljava/lang/Integer;", &[JValueGen::Int(*int)])
        );
        throw_exception!(
            env,
            env.set_object_array_element(&array, idx as i32 + 5, jint)
        )
    }

    let byte_buffer = env
        .call_static_method(
            byte_buffer,
            "wrap",
            "[B)Ljava/nio/ByteBuffer;",
            &[JValueGen::from(&buf_wrapper)],
        )
        .unwrap();
    let JValueGen::Object(byte_buffer) = byte_buffer else {
        throw_exception!(
            env,
            Err("ByteBuffer was not a ByteBuffer, this should never be thrown")
        )
    };
    throw_exception!(env, env.set_object_array_element(&array, 7, byte_buffer));

    array
}

pub fn try_extract_props(
    session: &GlobalSystemMediaTransportControlsSession,
) -> Result<PropsTuple, windows::core::Error> {
    let props = session.TryGetMediaPropertiesAsync()?.get()?;

    let title = props.Title()?.to_string();
    let artist = props.Artist()?.to_string();
    let subtitle = props.Subtitle()?.to_string();
    let album_name = props.AlbumTitle()?.to_string();
    let album_artist = props.AlbumArtist()?.to_string();
    let album_len = props.AlbumTrackCount()?;
    let track_number = props.TrackNumber()?;
    let icon = props.Thumbnail()?.OpenReadAsync()?.get()?;

    let size = icon.Size()?;
    let mut rust_buf = vec![0; size as usize];

    let buf = Buffer::Create(size as u32)?;
    let mut total = 0;
    while total != size {
        let read = icon
            .ReadAsync(&buf, size as u32, InputStreamOptions::None)?
            .get()?;
        let len = read.Length()? as usize;
        let data_reader = DataReader::FromBuffer(&read)?;
        data_reader.ReadBytes(&mut rust_buf[total as usize..total as usize + len])?;

        total += len as u64;
    }

    Ok((
        [
            title.into(),
            artist.into(),
            subtitle.into(),
            album_name.into(),
            album_artist.into(),
        ],
        [album_len, track_number],
        rust_buf.into(),
    ))
}

pub fn set_properties_changed_callback(
    gsmtcs: &GlobalSystemMediaTransportControlsSession,
) -> Result<(usize, Receiver<SongInfoResult>), Box<dyn Error>> {
    static REGISTERED: AtomicBool = AtomicBool::new(false);

    if !REGISTERED.load(std::sync::atomic::Ordering::Relaxed) {
        let handler = TypedEventHandler::new(properties_changed);

        gsmtcs.MediaPropertiesChanged(&handler)?;
        REGISTERED.store(true, std::sync::atomic::Ordering::Relaxed);
    };

    let (tx, rx) = channel();

    let idx = {
        // TODO: This shold replace anything that is `None`
        let mut queues = QUEUE.write()?;
        queues.push(Some(tx));
        queues.len() - 1
    };

    Ok((idx, rx))
}
