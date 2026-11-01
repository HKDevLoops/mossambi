(define-module (gnu packages mossambi)
  #:use-module (guix packages)
  #:use-module (guix download)
  #:use-module (guix build-system cargo)
  #:use-module ((guix licenses) #:prefix license:)
  #:use-module (gnu packages pkg-config)
  #:use-module (gnu packages video)
  #:use-module (gnu packages linux)
  #:use-module (gnu packages gl)
  #:use-module (gnu packages xorg))

(define-public mossambi
  (package
    (name "mossambi")
    (version "2.1.0")
    (source
      (origin
        (method url-fetch)
        (uri (string-append "https://github.com/mossambi/mossambi/archive/v"
                            version ".tar.gz"))
        (sha256
          (base32 "0000000000000000000000000000000000000000000000000000"))))
    (build-system cargo-build-system)
    (native-inputs
      (list pkg-config))
    (inputs
      (list ffmpeg
            alsa-lib
            mesa
            libxkbcommon
            wayland))
    (home-page "https://github.com/mossambi/mossambi")
    (synopsis "Zero-loss native file converter and Indian exam studio")
    (description
      "Mossambi provides CLI, TUI, and GUI floating radial widget interfaces for
zero-loss file transcoding, archive compression, and constraint-based Indian
exam photo/signature tuning.")
    (license (list license:expat license:asl2.0))))
