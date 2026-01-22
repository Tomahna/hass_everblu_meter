with import <nixpkgs> {};

mkShell rec {
  nativeBuildInputs = [
    cargo clippy rustc rustfmt rust-analyzer
  ];
}
