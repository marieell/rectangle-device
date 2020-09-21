// This code may not be used for any purpose. Be gay, do crime.

use std::error::Error;
use std::process::{Stdio, Child};
use crate::config;
use crate::sandbox::runtime;
use crate::sandbox::socket::SocketPool;
use crate::sandbox::types::ImageDigest;

pub fn default_image() -> ImageDigest {
    ImageDigest::parse(config::FFMPEG_CONTAINER_NAME, config::FFMPEG_CONTAINER_HASH).unwrap()
}

#[derive(Clone, Debug)]
pub struct TranscodeConfig {
    pub image: ImageDigest,
    pub args: Vec<String>,
    pub allow_networking: bool,
    pub segment_time: f32,
}

pub fn start(tc: TranscodeConfig, pool: &SocketPool) -> Result<Child, Box<dyn Error>> {

    if !runtime::image_exists(&tc.image)? {
        runtime::pull(&tc.image)?;
    }

    // Be specific about sandbox options
    let mut command = runtime::command();
    command
        .arg("run")
        .arg("--attach=stdout")
        .arg("--attach=stderr")
        .arg("--env-host=false")
        .arg("--read-only")
        .arg("--restart=no")
        .arg("--detach=false")
        .arg("--privileged=false");

    // If we are intentionally ingesting data from the network, this will use slirp (a usermode
    // tcp/ip emulator) to perform network access from usermode. Access to localhost is restricted,
    // but any other hosts are allowed. If networking is off, the sandbox has no network at all.

    if tc.allow_networking {
        command
            .arg("--net=slirp4netns")
            .arg("--dns-search=.");
    } else {
        command.arg("--net=none");
    }

    command
        .args(&pool.mount_args)
        .arg(tc.image.digest.as_str())
        .args(tc.args);

    // Additional args for ffmpeg

    command
        .arg("-nostats")
        .arg("-nostdin")
        .arg("-loglevel").arg("error")
        .arg("-f").arg("stream_segment")
        .arg("-segment_format").arg("mpegts")
        .arg("-segment_wrap").arg("1")
        .arg("-segment_time").arg(tc.segment_time.to_string())
        .arg("unix:///out/%d.ts");

    log::info!("starting, {:?}", command);

    Ok(command
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?)
}