use std::path::PathBuf;

#[derive(Debug, palc::Parser)]
pub struct Args {
    /// specify a password, optional
    #[arg(long, short = 'p')]
    pub password: Option<String>,

    /// add an QRCode overlap for password
    #[arg(long, short = 'Q')]
    pub qrcode_overlap: bool,

    /// has quiet zone of QR Code
    ///
    /// can be: `true`/`false`
    ///
    /// Quiet zone means the surrounding blank area
    #[arg(long, short = 'q', default_value_t = true)]
    pub has_quiet_zone: std::primitive::bool, // workaround: bypass `bool` match

    /// Position of QR code.
    ///
    /// Can be one of `top-left` (default), `top-right`, `bottom-left`, `bottom-right`, `center`
    ///
    /// will fallback to default on invalid input.
    #[arg(long, short = 'P')]
    pub qr_position: Option<String>,

    /// Color of QR Code foreground (the bar itself)
    ///
    /// format: CSS3 Color
    #[arg(long, default_value = "#000000ff")]
    pub qrcode_fg_color: String,
    /// Color of QR Code background (The blank background)
    ///
    /// format: CSS3 Color
    #[arg(long, default_value = "ffffffff")]
    pub qrcode_bg_color: String,

    /// target file. if enabled `qrcode_overlap, must be one of PNG, JPEG and WEBP.`
    pub img: PathBuf,
    pub path: Vec<PathBuf>,
}
