use lazy_static::lazy_static;
use rand::seq::SliceRandom;

lazy_static! {
    static ref PICS: Vec<&'static [u8]> = vec![
        include_bytes!("../../assets/pic.jpeg"),
        include_bytes!("../../assets/pic2.jpg"),
        include_bytes!("../../assets/pic3.jpg")
    ];
}

pub fn get_rand_image() -> Vec<u8> {
    PICS.choose(&mut rand::thread_rng()).unwrap().to_vec()
}
