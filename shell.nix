let
  mozillaOverlay =
    import (builtins.fetchGit {
      url = "https://github.com/mozilla/nixpkgs-mozilla.git";
      rev = "6eabade97bc28d707a8b9d82ad13ef143836736e";
    });
  pinned = builtins.fetchGit {
    # Descriptive name to make the store path easier to identify
    url = "https://github.com/nixos/nixpkgs/";
    # Commit hash for nixos-unstable as of 2020-04-26
    # `git ls-remote https://github.com/nixos/nixpkgs nixos-unstable`
    ref = "refs/heads/nixos-unstable";
    rev = "1fe6ed37fd9beb92afe90671c0c2a662a03463dd";
  };
  nixpkgs = import pinned { overlays = [ mozillaOverlay ]; };
  toolchain = with nixpkgs; (rustChannelOf { date = "2023-05-23"; channel = "nightly"; });
  rust-wasm = toolchain.rust.override {
    targets = [ "wasm32-unknown-unknown" ];
    extensions = ["rust-src"];
  };
in
with nixpkgs; pkgs.mkShell {
  buildInputs = [
    clang
    pkg-config
    rust-wasm
    openssl
  ];

  LIBCLANG_PATH = "${llvmPackages.libclang}/lib";
  PROTOC = "${protobuf}/bin/protoc";
  RUST_SRC_PATH = "${toolchain.rust-src}/lib/rustlib/src/rust/library/";
  ROCKSDB_LIB_DIR = "${rocksdb}/lib";

}
