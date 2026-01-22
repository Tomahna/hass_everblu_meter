with import <nixpkgs> {};

mkShell rec {
  nativeBuildInputs = [
    cargo rustc rustfmt rust-analyzer
  ];
}
