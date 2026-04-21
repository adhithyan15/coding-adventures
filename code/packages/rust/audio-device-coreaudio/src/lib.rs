//! # audio-device-coreaudio
//!
//! macOS Core Audio backend for the backend-neutral `audio-device-sink` crate.

use audio_device_sink::{AudioSink, AudioSinkError, PcmPlaybackBuffer, PlaybackReport};

/// Crate version, kept explicit for examples and integration tests.
pub const VERSION: &str = "0.1.0";

/// Human-readable backend name used in reports.
pub const BACKEND_NAME: &str = "coreaudio";

/// Core Audio implementation of the generic audio sink contract.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CoreAudioSink;

impl CoreAudioSink {
    /// Construct a default Core Audio sink.
    pub const fn new() -> Self {
        Self
    }

    /// Return the stable backend name.
    pub const fn backend_name(&self) -> &'static str {
        BACKEND_NAME
    }
}

impl AudioSink for CoreAudioSink {
    fn play_blocking(&self, buffer: &PcmPlaybackBuffer) -> Result<PlaybackReport, AudioSinkError> {
        if buffer.is_empty() {
            return Ok(PlaybackReport::for_buffer(buffer, BACKEND_NAME));
        }

        #[cfg(target_os = "macos")]
        {
            macos::play_blocking(buffer)?;
            Ok(PlaybackReport::for_buffer(buffer, BACKEND_NAME))
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = buffer;
            Err(AudioSinkError::unsupported_platform(
                "Core Audio playback is only available on macOS",
            ))
        }
    }
}

/// Convenience function for callers that do not need to store the sink.
pub fn play_default_output_blocking(
    buffer: &PcmPlaybackBuffer,
) -> Result<PlaybackReport, AudioSinkError> {
    CoreAudioSink::new().play_blocking(buffer)
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;
    use std::ptr;
    use std::sync::{Condvar, Mutex};
    use std::time::Duration;

    use audio_device_sink::{AudioSinkError, PcmPlaybackBuffer};

    type OSStatus = i32;
    type AudioQueueRef = *mut c_void;
    type AudioQueueBufferRef = *mut AudioQueueBuffer;

    const NO_ERR: OSStatus = 0;
    const AUDIO_FORMAT_LINEAR_PCM: u32 = 0x6c70_636d;
    const LINEAR_PCM_FORMAT_FLAG_IS_SIGNED_INTEGER: u32 = 1 << 2;
    const LINEAR_PCM_FORMAT_FLAG_IS_PACKED: u32 = 1 << 3;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct AudioStreamBasicDescription {
        mSampleRate: f64,
        mFormatID: u32,
        mFormatFlags: u32,
        mBytesPerPacket: u32,
        mFramesPerPacket: u32,
        mBytesPerFrame: u32,
        mChannelsPerFrame: u32,
        mBitsPerChannel: u32,
        mReserved: u32,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct AudioStreamPacketDescription {
        mStartOffset: i64,
        mVariableFramesInPacket: u32,
        mDataByteSize: u32,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct AudioQueueBuffer {
        mAudioDataBytesCapacity: u32,
        mAudioData: *mut c_void,
        mAudioDataByteSize: u32,
        mUserData: *mut c_void,
        mPacketDescriptionCapacity: u32,
        mPacketDescriptions: *mut AudioStreamPacketDescription,
        mPacketDescriptionCount: u32,
    }

    type AudioQueueOutputCallback =
        Option<unsafe extern "C" fn(*mut c_void, AudioQueueRef, AudioQueueBufferRef)>;

    #[link(name = "AudioToolbox", kind = "framework")]
    extern "C" {
        fn AudioQueueNewOutput(
            inFormat: *const AudioStreamBasicDescription,
            inCallbackProc: AudioQueueOutputCallback,
            inUserData: *mut c_void,
            inCallbackRunLoop: *mut c_void,
            inCallbackRunLoopMode: *mut c_void,
            inFlags: u32,
            outAQ: *mut AudioQueueRef,
        ) -> OSStatus;

        fn AudioQueueAllocateBuffer(
            inAQ: AudioQueueRef,
            inBufferByteSize: u32,
            outBuffer: *mut AudioQueueBufferRef,
        ) -> OSStatus;

        fn AudioQueueEnqueueBuffer(
            inAQ: AudioQueueRef,
            inBuffer: AudioQueueBufferRef,
            inNumPacketDescs: u32,
            inPacketDescs: *const AudioStreamPacketDescription,
        ) -> OSStatus;

        fn AudioQueueStart(inAQ: AudioQueueRef, inStartTime: *const c_void) -> OSStatus;
        fn AudioQueueStop(inAQ: AudioQueueRef, inImmediate: u8) -> OSStatus;
        fn AudioQueueDispose(inAQ: AudioQueueRef, inImmediate: u8) -> OSStatus;
    }

    struct PlaybackState {
        completed: Mutex<bool>,
        completed_changed: Condvar,
    }

    impl PlaybackState {
        fn new() -> Self {
            Self {
                completed: Mutex::new(false),
                completed_changed: Condvar::new(),
            }
        }

        fn mark_completed(&self) {
            if let Ok(mut completed) = self.completed.lock() {
                *completed = true;
                self.completed_changed.notify_all();
            }
        }

        fn wait_for_completion(&self, timeout: Duration) -> bool {
            let Ok(completed) = self.completed.lock() else {
                return false;
            };
            let Ok((completed, timeout_result)) =
                self.completed_changed
                    .wait_timeout_while(completed, timeout, |completed| !*completed)
            else {
                return false;
            };
            *completed || !timeout_result.timed_out()
        }
    }

    unsafe extern "C" fn playback_finished(
        user_data: *mut c_void,
        _queue: AudioQueueRef,
        _buffer: AudioQueueBufferRef,
    ) {
        if !user_data.is_null() {
            let state = &*(user_data as *const PlaybackState);
            state.mark_completed();
        }
    }

    pub fn play_blocking(buffer: &PcmPlaybackBuffer) -> Result<(), AudioSinkError> {
        let byte_count = buffer
            .sample_count()
            .checked_mul(buffer.format().sample_width_bytes())
            .ok_or_else(|| AudioSinkError::invalid_samples("PCM byte count overflowed"))?;
        let byte_count_u32 = u32::try_from(byte_count).map_err(|_| {
            AudioSinkError::invalid_samples(
                "Core Audio V1 buffers must fit in one AudioQueue buffer",
            )
        })?;

        let bytes = samples_to_le_bytes(buffer.samples());
        let format = audio_stream_description(buffer);
        let state = Box::new(PlaybackState::new());
        let state_ptr = Box::into_raw(state);

        let result =
            unsafe { play_with_audio_queue(&format, &bytes, byte_count_u32, buffer, state_ptr) };
        unsafe {
            drop(Box::from_raw(state_ptr));
        }
        result
    }

    unsafe fn play_with_audio_queue(
        format: &AudioStreamBasicDescription,
        bytes: &[u8],
        byte_count: u32,
        buffer: &PcmPlaybackBuffer,
        state_ptr: *mut PlaybackState,
    ) -> Result<(), AudioSinkError> {
        let mut queue: AudioQueueRef = ptr::null_mut();
        check_status(
            AudioQueueNewOutput(
                format,
                Some(playback_finished),
                state_ptr as *mut c_void,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                &mut queue,
            ),
            "AudioQueueNewOutput",
        )?;

        let mut audio_buffer: AudioQueueBufferRef = ptr::null_mut();
        let playback_result = (|| {
            check_status(
                AudioQueueAllocateBuffer(queue, byte_count, &mut audio_buffer),
                "AudioQueueAllocateBuffer",
            )?;
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                (*audio_buffer).mAudioData as *mut u8,
                bytes.len(),
            );
            (*audio_buffer).mAudioDataByteSize = byte_count;

            check_status(
                AudioQueueEnqueueBuffer(queue, audio_buffer, 0, ptr::null()),
                "AudioQueueEnqueueBuffer",
            )?;
            check_status(AudioQueueStart(queue, ptr::null()), "AudioQueueStart")?;

            let timeout = Duration::from_secs_f64(buffer.duration_seconds() + 2.0);
            let completed = (&*state_ptr).wait_for_completion(timeout);
            if completed {
                Ok(())
            } else {
                Err(AudioSinkError::backend_failure(
                    "Core Audio playback timed out before the buffer completion callback",
                ))
            }
        })();

        let _ = AudioQueueStop(queue, 1);
        let _ = AudioQueueDispose(queue, 1);
        playback_result
    }

    fn samples_to_le_bytes(samples: &[i16]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(samples.len() * 2);
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes
    }

    fn audio_stream_description(buffer: &PcmPlaybackBuffer) -> AudioStreamBasicDescription {
        let format = buffer.format();
        AudioStreamBasicDescription {
            mSampleRate: format.sample_rate_hz as f64,
            mFormatID: AUDIO_FORMAT_LINEAR_PCM,
            mFormatFlags: LINEAR_PCM_FORMAT_FLAG_IS_SIGNED_INTEGER
                | LINEAR_PCM_FORMAT_FLAG_IS_PACKED,
            mBytesPerPacket: 2,
            mFramesPerPacket: 1,
            mBytesPerFrame: 2,
            mChannelsPerFrame: format.channel_count as u32,
            mBitsPerChannel: format.bit_depth as u32,
            mReserved: 0,
        }
    }

    fn check_status(status: OSStatus, operation: &str) -> Result<(), AudioSinkError> {
        if status == NO_ERR {
            Ok(())
        } else {
            Err(AudioSinkError::backend_failure(format!(
                "{operation} failed with OSStatus {status}"
            )))
        }
    }
}
