{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = {
    flake-utils,
    naersk,
    nixpkgs,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = (import nixpkgs) {
          inherit system;
        };

        naersk' = pkgs.callPackage naersk {};
      in {
        packages.default = naersk'.buildPackage {
          src = ./.;
          nativeBuildInputs = with pkgs; [pkg-config tree];
          buildInputs = with pkgs; [openssl];

          postInstall = ''
            mkdir -p $out/share/zsh/site-functions
            cp completions/lecture-mgr.zsh $out/share/zsh/site-functions/_lecture-mgr
          '';
        };

        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [rustc cargo pkg-config];
          buildInputs = with pkgs; [openssl];
        };
      }
    );
}
