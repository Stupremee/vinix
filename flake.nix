{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };

      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        extensions = ["rust-src"];
      };

      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

      vinix = craneLib.buildPackage {
        src = craneLib.cleanCargoSource (craneLib.path ./.);

        doCheck = false;
      };
    in {
      checks = {
        inherit vinix;
      };

      formatter = pkgs.alejandra;

      packages.default = vinix;

      apps.default = flake-utils.lib.mkApp {
        drv = vinix;
      };

      devShells.default = pkgs.mkShell {
        inputsFrom = builtins.attrValues self.checks.${system};

        nativeBuildInputs = with pkgs; [
          alejandra
          rustToolchain
        ];
      };
    });
}
