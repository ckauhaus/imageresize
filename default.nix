{ pkgs ? import <nixpkgs> {} }:

with pkgs.lib;

pkgs.stdenv.mkDerivation rec {
  name = "imageresize";
  src = ./imageresize.py;
  nativeBuildInputs = with pkgs; [ makeWrapper ];
  propagatedBuildInputs = with pkgs; [
    imagemagick.out
    libjpeg.bin
    python3
  ];
  unpackPhase = ":";
  configurePhase = ":";
  dontBuild = true;
  dontStrip = true;
  dontPatchELF = true;
  installPhase = ''
    install -D -m 0755 $src $out/bin/imageresize
    wrapProgram $out/bin/imageresize \
      --prefix PATH : ${makeBinPath propagatedBuildInputs}
  '';
}
