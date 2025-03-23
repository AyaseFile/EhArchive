{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, fenix, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-darwin"
      ];

      forAllSystems =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f {
            inherit system;
            pkgs = import nixpkgs { inherit system; };
          }
        );
    in
    {
      devShells = forAllSystems (
        { pkgs, system, ... }:
        with pkgs;
        let
          rust_toolchain = fenix.packages.${system}.stable.withComponents [
            "cargo"
            "rustc"
            "rust-src"
            "clippy"
            "rustfmt"
          ];
        in
        {
          default = mkShell {
            nativeBuildInputs = [
              rust_toolchain
              pkg-config
            ];
            buildInputs = [
              openssl
            ];
            RUST_SRC_PATH = "${rust_toolchain}/lib/rustlib/src/rust/library";
          };
        }
      );
    };
}
