use ructe::{Ructe, RucteError};

fn main() -> Result<(), RucteError> {
    let mut ructe = Ructe::from_env()?;
    let mut statics = ructe.statics()?;
    statics.add_file("logo.png")?;
    statics.add_sass_file("style.scss")?;
    ructe.compile_templates("templates")
}
