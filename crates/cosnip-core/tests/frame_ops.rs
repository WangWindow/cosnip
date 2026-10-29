use cosnip_core::Frame;

fn frame_from_rows(rows: &[[[u8; 4]; 2]]) -> Frame {
    let mut rgba = Vec::new();
    for row in rows {
        for pixel in row {
            rgba.extend_from_slice(pixel);
        }
    }
    Frame::new(2, rows.len() as u32, rgba).unwrap()
}

fn solid(width: u32, height: u32, color: [u8; 4]) -> Frame {
    Frame::new(
        width,
        height,
        color.repeat(width as usize * height as usize),
    )
    .unwrap()
}

#[test]
fn identity_scale_preserves_pixels() {
    let frame = frame_from_rows(&[[[10, 20, 30, 255], [40, 50, 60, 255]]]);
    assert_eq!(frame.scaled(2, 1).unwrap(), frame);
}

#[test]
fn bilinear_upscale_keeps_corners_and_blends_center() {
    let frame = frame_from_rows(&[
        [[0, 0, 0, 255], [255, 0, 0, 255]],
        [[0, 255, 0, 255], [0, 0, 255, 255]],
    ]);
    let scaled = frame.scaled(4, 4).unwrap();
    assert_eq!(scaled.width(), 4);
    assert_eq!(scaled.height(), 4);
    assert_eq!(scaled.pixel(0, 0), Some([0, 0, 0, 255]));
    assert_eq!(scaled.pixel(3, 0), Some([255, 0, 0, 255]));
    assert_eq!(scaled.pixel(0, 3), Some([0, 255, 0, 255]));
    assert_eq!(scaled.pixel(3, 3), Some([0, 0, 255, 255]));
    let center = scaled.pixel(1, 1).unwrap();
    assert!(center[0] > 0 && center[1] > 0 && center[2] > 0);
}

#[test]
fn scale_rejects_empty_dimensions() {
    let frame = solid(2, 2, [1, 2, 3, 255]);
    assert!(frame.scaled(0, 2).is_err());
    assert!(frame.scaled(2, 0).is_err());
}

#[test]
fn blit_copies_and_clips() {
    let mut canvas = solid(4, 4, [0, 0, 0, 255]);
    let source = solid(2, 2, [9, 8, 7, 255]);
    canvas.blit(&source, 1, 1);
    assert_eq!(canvas.pixel(0, 0), Some([0, 0, 0, 255]));
    assert_eq!(canvas.pixel(1, 1), Some([9, 8, 7, 255]));
    assert_eq!(canvas.pixel(2, 2), Some([9, 8, 7, 255]));
    assert_eq!(canvas.pixel(3, 3), Some([0, 0, 0, 255]));

    canvas.blit(&source, -1, 3);
    assert_eq!(canvas.pixel(0, 3), Some([9, 8, 7, 255]));
    assert_eq!(canvas.pixel(0, 2), Some([0, 0, 0, 255]));
}
