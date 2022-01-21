// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

//! This file is a rustic interpretation of the the [PipeWire Tutorial 5][tut]
//!
//! tut: https://docs.pipewire.org/page_tutorial5.html

use pipewire as pw;
use pw::prelude::*;
use pw::{properties, spa};

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "streams", about = "Stream example")]
struct Opt {
    #[structopt(short, long, help = "The target object id to connect to")]
    target: Option<u32>,
}

pub fn main() -> Result<(), pw::Error> {
    pw::init();

    let opt = Opt::from_args();

    let mainloop = pw::MainLoop::new()?;

    let stream = pw::stream::Stream::<i32>::with_user_data(
        &mainloop,
        "video-test",
        properties! {
            *pw::keys::MEDIA_TYPE => "Video",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Camera",
        },
        0,
    )
    .state_changed(|old, new| {
        println!("State changed: {:?} -> {:?}", old, new);
    })
    .process(|stream, frame_count| {
        println!("On frame");
        match stream.dequeue_buffer() {
            None => println!("No buffer received"),
            Some(mut buffer) => {
                let datas = buffer.datas_mut();
                println!("Frame {}. Got {} datas.", frame_count, datas.len());
                *frame_count += 1;
                // TODO: get the frame size and display it
            }
        }
    })
    // TODO: connect params_changed
    .create()?;

    println!("Created stream {:#?}", stream);

    // TODO: set params

    stream.connect(
        spa::Direction::Input,
        opt.target,
        pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
        &mut [],
    )?;

    println!("Connected stream");

    mainloop.run();

    unsafe { pw::deinit() };

    Ok(())
}
