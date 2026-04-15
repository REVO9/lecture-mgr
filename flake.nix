{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {inherit system;};
  in {
    packages.${system}.default = pkgs.rustPlatform.buildRustPackage rec {
      pname = "lecture-mgr";
      version = "0.1.0";
      src = ./.;
      cargoLock = {
        lockFile = ./Cargo.lock;
      };
      nativeBuildInputs = with pkgs; [pkg-config];
      buildInputs = with pkgs; [openssl];
    };

    devShells.${system}.default = pkgs.mkShell{
      nativeBuildInputs = with pkgs; [rustc cargo pkg-config  ];
      buildInputs = with pkgs; [openssl];
    };
  };
}
