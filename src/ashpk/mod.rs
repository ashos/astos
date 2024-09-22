//#[cfg(feature = "apk")]
// APK package manager
//pub mod apk;

#[cfg(feature = "apt")]
// APT package manager
pub mod apt;

#[cfg(feature = "dnf")] //TODO
// DNF package manager
pub mod dnf;

//#[cfg(feature = "pkgtool")] // TODO
// PKGTOOL
//pub mod pkgtool;

//#[cfg(feature = "portage")] // TODO
// Portage package manager
//pub mod portage;

//#[cfg(feature = "xbps")] // TODO
// XBPS package manager
//pub mod xbps;

// Pacman package manager
#[cfg(feature = "pacman")]
pub mod pacman;
