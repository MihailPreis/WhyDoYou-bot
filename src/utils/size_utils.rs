/// Resizing abstract size based on aspect ratio of the original size
///
/// Parameters:
///  - root_w: 'From' width
///  - root_h: 'From' height
///  - target_w: 'To' width
///  - target_h: 'To' height
///
/// Return: new size as tuple of width and height
pub fn aspect_resize(root_w: u32, root_h: u32, target_w: u32, target_h: u32) -> (u32, u32) {
    let ratio: f32 = (target_w as f32 / root_w as f32).min(target_h as f32 / root_h as f32);
    return (
        (root_w as f32 * ratio) as u32,
        (root_h as f32 * ratio) as u32,
    );
}
