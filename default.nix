# Needs at least Rust 1.34 (NixOS 19.09)
{ pkgs ? import (builtins.fetchTarball {
    name = "nixos-unstable";
    url = https://github.com/NixOS/nixpkgs-channels/archive/bc9df0f66110039e495b6debe3a6cda4a1bb0fed.tar.gz;
    sha256 = "0y2w259j0vqiwjhjvlbsaqnp1nl2zwz6sbwwhkrqn7k7fmhmxnq1";
  }) {}
}:

with builtins;
with pkgs;

rustPlatform.buildRustPackage rec {
  name = "imageresize-${version}";
  version = "0.2.1";
  src = lib.cleanSourceWith {
    filter = (name: _: baseNameOf name != "target");
    src = (lib.cleanSource ./.);
  };
  cargoSha256 = "01nl5gy5v27w76di51dwns3dmgysvbspg786fgfszprgnq9mc0p3";
  doCheck = false;

  nativeBuildInputs = [ pkgconfig ];
  propagatedBuildInputs = [ gexiv2.dev glib.dev ];
}
