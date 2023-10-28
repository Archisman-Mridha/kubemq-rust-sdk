#![allow(non_snake_case)]

use std::{path::PathBuf, env};

fn main( ) {
  let outputDirectory = PathBuf::from(env::var("OUT_DIR").unwrap( ));

  tonic_build::configure( )
    .protoc_arg("--experimental_allow_proto3_optional")
    .build_server(false)
    .build_client(true)
    .file_descriptor_set_path(outputDirectory.join("kubemq.descriptor.bin"))
    .compile(&["protos/kubemq.proto"], &[""])
  .unwrap( );
}