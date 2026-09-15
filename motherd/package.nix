{ rustPlatform, lib }:
rustPlatform.buildRustPackage {
  pname = "motherd";
  version = "0.1.0";
  src = lib.cleanSource ./.;
  cargoLock.lockFile = ./Cargo.lock;
  meta.description = "AIOS dead-core motherd";
}
