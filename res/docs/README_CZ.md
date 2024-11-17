<p align="center">
  <img src="res/logos/logo.svg" alt="AshOS">
  <br>
  [<a href="res/docs/README_CZ.md">Čeština</a>] | [<a href="res/docs/README_zh-CN.md">中文</a>] | [<a href="res/docs/README_FA.md">پارسی</a>]
  <br>
  <b>We need your help to translate this README. <a href="https://github.com/i2/ashos-dev/tree/main/res/docs">Look here!</a></b>
</p>

# AshOS (Any Snapshot Hierarchical OS)
### An immutable tree-shaped meta-distribution using snapshots

## What is AshOS?

AshOS is a unique meta-distribution that:
* aims to bring immutability even to distros that do not have this very useful feature i.e. Arch Linux, Gentoo, etc.
* wraps around *any* Linux distribution that can be bootstrapped (pretty much any major distribution)
* targets to become a universal installer for different distros and different Desktop Environments/Window Managers
* can install, deploy and multi-boot any number of distros

Initially inspired by Arch Linux, AshOS uses an immutable (read-only) root filesystem to set itself apart from any other distro out there.
Software is installed and configured into individual snapshot trees, which can then be deployed and booted into.
It does not invent yet another package format or package manager, but instead relies on the native package manager for instance [pacman](https://wiki.archlinux.org/title/pacman) from Arch.

Ashes are one of the oldest trees in the world and they inspired naming AshOS.

In AshOS, there are several keywords:
* Vanilla: we try to be as close to the "vanilla" version of target distribution that is being installed.
* Minimalism: we adhere to a lego build system. Start small and build as complex a system as you would like. The main focus of development is on having a solid minimal installed snapshot, based on which user can have infinite immutable permutations!
* Generality: We strive to cater for the most common denominator between distros and architectures (x64, aarch64, sparc, etc). As such, when there is a choice between convenience and comprehensiveness/generality, we go with the latter. To clarify with an example, it might be easier to use grub-btrfs instead of implementing our own GRUB update mechanism, but because that particular package might not be readily available in all distros, we develop an AshOS specific solution. This way, we can potentially cater to any distro in future!

**This has several advantages:**

* Security
  * Even if running an application with eleveted permissions, it cannot replace system libraries with malicious versions
* Stability and reliability
  * Due to the system being mounted as read only, it's not possible to accidentally overwrite system files
  * If the system runs into issues, you can easily rollback the last working snapshot within minutes
  * Atomic updates - Updating your system all at once is more reliable
  * Thanks to the snapshot feature, AshOS can ship cutting edge software without becoming unstable
  * AshOS needs little maintenance, as it has a built in fully automatic update tool that creates snapshots before updates and automatically checks if the system upgraded properly before deploying the new snapshot
* Configurability
  * With the snapshots organised into a tree, you can easily have multiple different configurations of your software available, with varying packages, without any interference
  * For example: you can have a single Gnome desktop installed and then have 2 snapshots on top - one with your video games, with the newest kernel and drivers, and the other for work, with the LTS kernel and more stable software, you can then easily switch between these depending on what you're trying to do
  * You can also easily try out software without having to worry about breaking your system or polluting it with unnecessary files, for example you can try out a new desktop environment in a snapshot and then delete the snapshot after, without modifying your main system at all
  * This can also be used for multi-user systems, where each user has a completely separate system with different software, and yet they can share certain packages such as kernels and drivers
  * AshOS allows you to install software by chrooting into snapshots, therefore (for example in Arch flavor) you can use software such as the AUR to install additional packages
  * AshOS is, very customizable, you can choose exactly which software you want to use (just like Arch Linux)

* Thanks to its reliabilty and automatic upgrades, AshOS is well suitable for single use or embedded devices
* It also makes for a good workstation or general use distribution utilizing development containers and flatpak for desktop applications

## AshOS Wiki?
More information can be found on [AshOSWiki](https://github.com/ashos/ashos/wiki).
