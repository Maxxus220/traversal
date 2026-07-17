{
  description = "traversal – tag cross-referencer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/1e78637806c14b81a1e8dccadf00be7e93dda457";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            clippy
            rustc
            rustfmt
            nodejs
          ];

          RUST_EDITION = "2024";
        };
      });
}
