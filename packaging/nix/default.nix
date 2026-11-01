{ lib
, rustPlatform
, pkg-config
, alsa-lib
, libxkbcommon
, wayland
, mesa
, libGL
, ffmpeg
}:

rustPlatform.buildRustPackage rec {
  pname = "mossambi";
  version = "2.1.0";

  src = lib.cleanSource ../..;

  cargoLock = {
    lockFile = ../../Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
  ];

  buildInputs = [
    alsa-lib
    libxkbcommon
    wayland
    mesa
    libGL
    ffmpeg
  ];

  postInstall = ''
    install -Dm644 packaging/assets/mossambi.desktop $out/share/applications/mossambi.desktop
  '';

  meta = with lib; {
    description = "Zero-loss native file converter, archive suite, and Indian exam form tuner";
    homepage = "https://github.com/mossambi/mossambi";
    license = with licenses; [ mit asl20 ];
    maintainers = [ ];
    mainProgram = "mossambi";
    platforms = platforms.unix;
  };
}
