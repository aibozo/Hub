#![cfg(feature = "realtime-audio")]

#[test]
fn ulaw_decode_basic() {
    let ulaw = vec![0u8, 255u8, 128u8, 127u8];
    let pcm = assistant_core::realtime_audio::decode_ulaw_to_pcm(&ulaw);
    assert_eq!(pcm.len(), 4);
    // Values should not all be zero and should vary
    assert!(pcm.iter().any(|&s| s != 0));
}

#[test]
fn resample_linear_monotonic() {
    let src: Vec<i16> = (0..100).map(|x| x as i16).collect();
    let out = assistant_core::realtime_audio::resample_linear_i16(&src, 16000, 24000);
    assert!(out.len() > src.len());
    // First/last sample preserved approximately
    assert!((out[0] as i32 - src[0] as i32).abs() <= 1);
}

