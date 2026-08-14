use std::{
    error::Error,
    sync::{
        atomic::AtomicBool,
        mpsc::{channel, Receiver},
    },
};

use jni::descriptors::Desc;
use jni::objects::{JByteBuffer, JClass};
use jni::refs::Reference;
use jni::{jni_sig, jni_str, objects::{JObject, JObjectArray}, Env, JValue, JValueOwned};
use windows::{
    Foundation::TypedEventHandler,
    Media::Control::{GlobalSystemMediaTransportControlsSession, MediaPropertiesChangedEventArgs},
    Storage::Streams::{Buffer, DataReader, InputStreamOptions},
};

use crate::{PropsTuple, SongInfoResult, QUEUE};

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
    env: &mut Env<'local>,
    info: PropsTuple,
) -> JObjectArray<'local> {
    let (strings, ints, buf) = info;

    let byte_buffer = Desc::<JClass>::lookup(JByteBuffer::class_name(), env).unwrap();
    let integer = Desc::<JClass>::lookup(jni_str!("java/lang/Integer"), env).unwrap();
    let buf_wrapper = env.byte_array_from_slice(&buf).unwrap();

    let array = JObjectArray::<JObject>::new(env, 8, JObject::default()).unwrap();

    for (idx, string) in strings.iter().enumerate() {
        let jstring = env.new_string(string).unwrap();
        array.set_element(env, idx, jstring).unwrap();
    }

    for (idx, int) in ints.iter().enumerate() {
        let jint = env.call_static_method(&integer, jni_str!("valueOf"), jni_sig!("(I)Ljava/lang/Integer;"), &[JValue::Int(*int)]).unwrap().into_object().unwrap();
        array.set_element(env, idx + 5, jint).unwrap();
    }

    let byte_buffer = env.call_static_method(byte_buffer, jni_str!("wrap"), jni_sig!("([B)Ljava/nio/ByteBuffer;"), &[JValue::from(&buf_wrapper)]).unwrap();
    let JValueOwned::Object(byte_buffer) = byte_buffer else {
        panic!("ByteBuffer was not a ByteBuffer, this should never be thrown")
    };

    array.set_element(env, 7, byte_buffer).unwrap();

    array
}

pub fn try_extract_props(
    session: &GlobalSystemMediaTransportControlsSession,
) -> Result<PropsTuple, windows::core::Error> {
    let props = session.TryGetMediaPropertiesAsync()?.join()?;

    let title = props.Title()?.to_string();
    let artist = props.Artist()?.to_string();
    let subtitle = props.Subtitle()?.to_string();
    let album_name = props.AlbumTitle()?.to_string();
    let album_artist = props.AlbumArtist()?.to_string();
    let album_len = props.AlbumTrackCount()?;
    let track_number = props.TrackNumber()?;
    let icon = props.Thumbnail()?.OpenReadAsync()?.join()?;

    let size = icon.Size()?;
    let mut rust_buf = vec![0; size as usize];

    let buf = Buffer::Create(size as u32)?;
    let mut total = 0;
    while total != size {
        let read = icon
            .ReadAsync(&buf, size as u32, InputStreamOptions::None)?
            .join()?;
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

pub fn try_get_timeline_props(
    session: &GlobalSystemMediaTransportControlsSession,
) -> Result<[i64; 6], windows::core::Error> {
    let props = session.GetTimelineProperties()?;

    let start_time = props.StartTime()?.Duration;
    let end_time = props.EndTime()?.Duration;
    let pos = props.Position()?.Duration;
    let last_updated = props.LastUpdatedTime()?.UniversalTime;
    let max_seek_time = props.MaxSeekTime()?.Duration;
    let min_seek_time = props.MinSeekTime()?.Duration;

    Ok([
        start_time,
        end_time,
        pos,
        last_updated,
        max_seek_time,
        min_seek_time,
    ])
}
