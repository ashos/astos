use crate::{check_profile, check_mutability, chr_delete, chroot_exec, get_current_snapshot, get_tmp,
            grub, immutability_disable, immutability_enable, is_system_pkg, is_system_locked,
            post_transactions, prepare, remove_dir_content, snapshot_config_get, sync_time};

use configparser::ini::{Ini, WriteOptions};
use rustix::path::Arg;
use std::fs::{File, metadata, OpenOptions, read_dir};
use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Write};
use std::path::Path;
use std::process::{Command, ExitStatus};
use tempfile::TempDir;
use users::get_user_by_name;
use users::os::unix::UserExt;

// Check if AUR is setup right
pub fn aur_check(snapshot: &str) -> bool {
    let options = snapshot_config_get(snapshot);
    if options["aur"] == "True" {
        let aur = true;
        return aur;
    } else if options["aur"] == "False" {
        let aur = false;
        return aur;
    } else {
        panic!("Please insert valid value for aur in /.snapshots/etc/etc-{}/ash/ash.conf", snapshot);
    }
}

// Noninteractive update
pub fn auto_upgrade(snapshot: &str) -> Result<(), Error> {
    // Make sure snapshot exists
    if !Path::new(&format!("/.snapshots/rootfs/snapshot-{}", snapshot)).try_exists()? {
        return Err(Error::new(
            ErrorKind::NotFound, format!("Cannot upgrade as snapshot {} doesn't exist.", snapshot)));

    } else {
        // Required in virtualbox, otherwise error in package db update
        sync_time()?;

        // Prepare snapshot
        prepare(snapshot)?;

        // Avoid invalid or corrupted package (PGP signature) error
        Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                              .args(["pacman", "-Sy", "--noconfirm", "archlinux-keyring"])
                              .status().unwrap();

        if !aur_check(snapshot) {
            // Use pacman
            let excode = Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                                               .args(["pacman", "--noconfirm", "-Syyu"]).status()?;
            if excode.success() {
                post_transactions(snapshot)?;
                let mut file = OpenOptions::new().write(true)
                                                 .create(true)
                                                 .truncate(true)
                                                 .open("/.snapshots/ash/upstate")?;
                file.write_all("0 ".as_bytes())?;
                let mut file = OpenOptions::new().append(true)
                                                 .open("/.snapshots/ash/upstate")?;
                let date = Command::new("date").output()?;
                file.write_all(format!("\n{}", &date.stdout.to_string_lossy().as_str()?).as_bytes())?;
            } else {
                chr_delete(snapshot)?;
                let mut file = OpenOptions::new().write(true)
                                                 .create(true)
                                                 .truncate(true)
                                                 .open("/.snapshots/ash/upstate")?;
                file.write_all("1 ".as_bytes())?;
                let mut file = OpenOptions::new().append(true)
                                                 .open("/.snapshots/ash/upstate")?;
                let date = Command::new("date").output()?;
                file.write_all(format!("\n{}", &date.stdout.to_string_lossy().as_str()?).as_bytes())?;
                return Err(Error::new(ErrorKind::Other,
                                      "Failed to upgrade."));
            }
        } else {
            // Use paru if aur is enabled
            let args = format!("paru -Syyu --noconfirm");
            let excode = Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                                               .args(["su", "aur", "-c", &args])
                                               .status().unwrap();
            if excode.success() {
                post_transactions(snapshot)?;
                let mut file = OpenOptions::new().write(true)
                                                 .create(true)
                                                 .truncate(true)
                                                 .open("/.snapshots/ash/upstate")?;
                file.write_all("0 ".as_bytes())?;
                let mut file = OpenOptions::new().append(true)
                                                 .open("/.snapshots/ash/upstate")?;
                let date = Command::new("date").output()?;
                file.write_all(format!("\n{}", &date.stdout.to_string_lossy().as_str()?).as_bytes())?;
            } else {
                chr_delete(snapshot)?;
                let mut file = OpenOptions::new().write(true)
                                                 .create(true)
                                                 .truncate(true)
                                                 .open("/.snapshots/ash/upstate")?;
                file.write_all("1 ".as_bytes())?;
                let mut file = OpenOptions::new().append(true)
                                                 .open("/.snapshots/ash/upstate")?;
                let date = Command::new("date").output()?;
                file.write_all(format!("\n{}", &date.stdout.to_string_lossy().as_str()?).as_bytes())?;
                return Err(Error::new(ErrorKind::Other,
                                      "Failed to upgrade."));
            }
        }
    }
    Ok(())
}

// Reinstall base packages in snapshot
pub fn bootstrap(snapshot: &str) -> Result<(), Error> {
    // tmp database
    let tmp_db = TempDir::new_in("/.snapshots/tmp/")?;

    let excode = Command::new("sh")
        .arg("-c")
        .arg(format!("pacman --dbpath {} -r /.snapshots/rootfs/snapshot-chr{} -Sy --noconfirm --overwrite '*' base",
                     tmp_db.path().to_str().unwrap(),snapshot)).status()?;
    if !excode.success() {
        return Err(Error::new(ErrorKind::Other,
                              format!("Failed to install base packages in snapshot {} chroot.", snapshot)));
    }

    let paru_install = Command::new("sh")
        .arg("-c")
        .arg(format!("su aur -c 'paru --dbpath {} -r /.snapshots/rootfs/snapshot-chr{} -Sy --noconfirm --overwrite \"*\" paru'", //TODO replace paru with ash
                     tmp_db.path().to_str().unwrap(),snapshot)).status()?;
    if !paru_install.success() {
        return Err(Error::new(ErrorKind::Other,
                              format!("Failed to install paru package in snapshot {} chroot.", snapshot)));
    }

    // Copy pacman database from tmp
    remove_dir_content(&format!("/.snapshots/rootfs/snapshot-chr{}/var/lib/pacman", snapshot))?;
    Command::new("cp").args(["-r", "--reflink=auto"])
                      .arg(format!("{}/.", tmp_db.path().to_str().unwrap()))
                      .arg(format!("/.snapshots/rootfs/snapshot-chr{}/var/lib/pacman", snapshot)).status()?;
    Ok(())
}

// Copy cache of downloaded packages to shared
pub fn cache_copy(snapshot: &str, prepare: bool) -> Result<(), Error> {
    let tmp = get_tmp();
    if prepare {
        Command::new("cp").args(["-n", "-r", "--reflink=auto"])
                          .arg(format!("/.snapshots/rootfs/snapshot-{}/var/cache/pacman/pkg/.", snapshot))
                          .arg(format!("/.snapshots/rootfs/snapshot-chr{}/var/cache/pacman/pkg", tmp))
                          .output().unwrap();
    } else {
        Command::new("cp").args(["-n", "-r", "--reflink=auto"])
                          .arg(format!("/.snapshots/rootfs/snapshot-chr{}/var/cache/pacman/pkg/.", snapshot))
                          .arg(format!("/.snapshots/rootfs/snapshot-{}/var/cache/pacman/pkg", tmp))
                          .output().unwrap();
    }
    Ok(())
}

// Clean pacman cache
pub fn clean_cache(snapshot: &str) -> Result<(), Error> {
    if Path::new(&format!("/.snapshots/rootfs/snapshot-chr{}/var/cache/pacman/pkg", snapshot)).try_exists().unwrap() {
        remove_dir_content(&format!("/.snapshots/rootfs/snapshot-chr{}/var/cache/pacman/pkg", snapshot))?;
    }
    Ok(())
}

// Fix signature invalid error
pub fn fix_package_db(snapshot: &str) -> Result<(), Error> {
    // Make sure snapshot does exist
    if !Path::new(&format!("/.snapshots/rootfs/snapshot-{}", snapshot)).try_exists()? && !snapshot.is_empty() {
        return Err(Error::new(ErrorKind::NotFound,
                              format!("Cannot fix package man database as snapshot {} doesn't exist.", snapshot)));

        // Make sure snapshot is not in use
        } else if Path::new(&format!("/.snapshots/rootfs/snapshot-chr{}", snapshot)).try_exists()? {
        return Err(
            Error::new(ErrorKind::Unsupported,
                       format!("Snapshot {} appears to be in use. If you're certain it's not in use, clear lock with 'ash unlock -s {}'.",
                               snapshot,snapshot)));

    } else if snapshot.is_empty() && get_current_snapshot() == "0" {
        // Base snapshot unsupported
        return Err(Error::new(ErrorKind::Unsupported, format!("Snapshot 0 (base) should not be modified.")));

    } else if snapshot == "0" {
        // Base snapshot unsupported
        return Err(Error::new(ErrorKind::Unsupported, format!("Snapshot 0 (base) should not be modified.")));

    } else {
        let run_chroot: bool;
        // If snapshot is current running
        run_chroot = if snapshot.is_empty() {
            false
        } else {
            true
        };

        // Snapshot is mutable so do not make it immutable after fixdb is done
        let flip = if check_mutability(snapshot) {
            false
        } else {
            if immutability_disable(snapshot).is_ok() {
                println!("Snapshot {} successfully made mutable.", snapshot);
            }
            true
        };

        // Fix package database
        if run_chroot {
            prepare(snapshot)?;
        }
        let mut cmds: Vec<String> = Vec::new();
        let username = std::env::var_os("SUDO_USER").unwrap();
        let user = get_user_by_name(&username).unwrap();
        let home_dir = user.home_dir();
        let home = home_dir.to_str().unwrap();
        if run_chroot {
            let etc_gnupg = format!("/.snapshots/rootfs/snapshot-chr{}/etc/pacman.d/gnupg", snapshot);
            if Path::new(&etc_gnupg).try_exists()? && read_dir(&etc_gnupg)?.count() > 0 {
                cmds.push(format!("rm -rf /etc/pacman.d/gnupg"));
            }
            if Path::new(&format!("{}/.gnupg", home)).try_exists()? && read_dir(&format!("{}/.gnupg", home))?.count() > 0 {
                cmds.push(format!("rm -rf {}/.gnupg", home));
            }
            if Path::new("/var/lib/pacman/sync").try_exists()? && read_dir("/var/lib/pacman/sync")?.count() > 0 {
                cmds.push(format!("rm -r /var/lib/pacman/sync/*"));
            }
            cmds.push(format!("pacman -Syy"));
            cmds.push(format!("sudo -u {} gpg --refresh-keys", username.to_str().unwrap()));
            cmds.push(format!("pacman-key --init"));
            cmds.push(format!("pacman-key --populate archlinux"));
            cmds.push(format!("pacman -Syvv --noconfirm archlinux-keyring"));
        } else {
            if Path::new("/etc/pacman.d/gnupg").try_exists()? && read_dir("/etc/pacman.d/gnupg")?.count() > 0 {
                cmds.push(format!("rm -rf /etc/pacman.d/gnupg"));
            }
            if Path::new(&format!("{}/.gnupg", home)).try_exists()? && read_dir(&format!("{}/.gnupg", home))?.count() > 0 {
                cmds.push(format!("rm -rf {}/.gnupg", home));
            }
            if Path::new("/var/lib/pacman/sync").try_exists()? && read_dir("/var/lib/pacman/sync")?.count() > 0 {
                cmds.push(format!("rm -r /var/lib/pacman/sync/*"));
            }
            cmds.push(format!("sudo -u {} gpg --refresh-keys", username.to_str().unwrap()));
            cmds.push(format!("pacman-key --init"));
            cmds.push(format!("pacman-key --populate archlinux"));
        }
        for cmd in cmds {
            if run_chroot {
                let excode = Command::new("sh").arg("-c")
                                                .arg(format!("chroot /.snapshots/rootfs/snapshot-chr{} {}",
                                                             snapshot,&cmd)).status()?;
                if !excode.success() {
                    return Err(Error::new(ErrorKind::Other,
                                          format!("Run command {} failed.", &cmd)));
                }
            } else {
                let excode = Command::new("sh").arg("-c")
                                  .arg(&cmd).status()?;
                if !excode.success() {
                    return Err(Error::new(ErrorKind::Other,
                                          format!("Run command {} failed.", &cmd)));
                }
            }
        }
        if snapshot.is_empty() {
            let snapshot = get_current_snapshot();
            prepare(&snapshot)?;
            refresh_helper(&snapshot).expect("Refresh failed.");
        }

        // Return snapshot to immutable after fixdb is done if snapshot was immutable
        if flip {
            if immutability_enable(snapshot).is_ok() {
                println!("Snapshot {} successfully made immutable.", snapshot);
            }
        }
    }
    Ok(())
}

// Install atomic-operation
pub fn install_package_helper(snapshot:&str, pkgs: &Vec<String>, noconfirm: bool) -> Result<(), Error> {
    prepare(snapshot)?;
    //Profile configurations
    let cfile = format!("/.snapshots/rootfs/snapshot-chr{}/etc/ash/profile", snapshot);
    let mut profconf = Ini::new_cs();
    profconf.set_comment_symbols(&['#']);
    profconf.set_multiline(true);
    let mut write_options = WriteOptions::default();
    write_options.blank_lines_between_sections = 1;
    // Load profile
    profconf.load(&cfile).unwrap();

    for pkg in pkgs {
        let mut pkgs_list: Vec<String> = Vec::new();
        if profconf.sections().contains(&"profile-packages".to_string()) {
            for pkg in profconf.get_map().unwrap().get("profile-packages").unwrap().keys() {
                pkgs_list.push(pkg.to_string());
            }
        }
        // This extra pacman check is to avoid unwantedly triggering AUR if package is official
        let pacman_si_arg = format!("pacman -Si {}", pkg);
        let excode = Command::new("sh").arg("-c")
                                       .arg(format!("chroot /.snapshots/rootfs/snapshot-chr{} {}", snapshot,pacman_si_arg))
                                       .output()?; // --sysroot
        let pacman_sg_arg = format!("pacman -Sg {}", pkg);
        let excode_group = Command::new("sh").arg("-c")
                                             .arg(format!("chroot /.snapshots/rootfs/snapshot-chr{} {}", snapshot,pacman_sg_arg))
                                             .output()?;
        if excode.status.success() || excode_group.status.success() {
            let pacman_args = if noconfirm {
                format!("pacman -S --noconfirm --needed --overwrite '/var/*' {}", pkg)
            } else {
                format!("pacman -S --needed --overwrite '/var/*' {}", pkg)
            };
            let excode = Command::new("sh").arg("-c")
                                            .arg(format!("chroot /.snapshots/rootfs/snapshot-chr{} {}", snapshot,pacman_args))
                                            .status()?;
            if !excode.success() {
                return Err(Error::new(ErrorKind::Other,
                                      format!("Failed to install {}", pkg)));
            // Add to profile-packages if not system package
            } else if !pkgs_list.contains(pkg) && !is_system_pkg(&profconf, pkg.to_string()) {
                pkgs_list.push(pkg.to_string());
                for key in pkgs_list {
                    profconf.remove_key("profile-packages", &key);
                    profconf.set("profile-packages", &key, None);
                }
                profconf.pretty_write(&cfile, &write_options)?;
            }
        } else if aur_check(snapshot) {
            // Use paru if aur is enabled
            let paru_args = if noconfirm {
                format!("paru -S --noconfirm --needed --overwrite '/var/*' {}", pkg)
            } else {
                format!("paru -S --needed --overwrite '/var/*' {}", pkg)
            };
            let excode = Command::new("chroot")
                .arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                .args(["su", "aur", "-c"])
                .arg(&paru_args)
                .status()?;
            if !excode.success() {
                return Err(Error::new(ErrorKind::Other,
                                      format!("Failed to install {}", pkg)));
            // Add to profile-packages if not system package
            } else if !pkgs_list.contains(pkg) && !is_system_pkg(&profconf, pkg.to_string()) {
                pkgs_list.push(pkg.to_string());
                pkgs_list.sort();
                for key in pkgs_list {
                    profconf.remove_key("profile-packages", &key);
                    profconf.set("profile-packages", &key, None);
                }
                profconf.pretty_write(&cfile, &write_options)?;
            }
        } else if !aur_check(snapshot) {
            return Err(Error::new(ErrorKind::NotFound,
                                  "Please enable AUR."));
        }
    }
    Ok(())
}

// Install atomic-operation
pub fn install_package_helper_chroot(snapshot:&str, pkgs: &Vec<String>, noconfirm: bool) -> Result<(), Error> {
    for pkg in pkgs {
        // This extra pacman check is to avoid unwantedly triggering AUR if package is official
        let pacman_si_arg = format!("pacman -Si {}", pkg);
        let excode = Command::new("sh").arg("-c")
                                       .arg(format!("chroot /.snapshots/rootfs/snapshot-chr{} {}", snapshot,pacman_si_arg))
                                       .output()?; // --sysroot
        let pacman_sg_arg = format!("pacman -Sg {}", pkg);
        let excode_group = Command::new("sh").arg("-c")
                                             .arg(format!("chroot /.snapshots/rootfs/snapshot-chr{} {}", snapshot,pacman_sg_arg))
                                             .output()?;
        if excode.status.success() || excode_group.status.success() {
            let pacman_args = if noconfirm {
                format!("pacman -Sy --noconfirm --needed --overwrite \"*\" {}", pkg)
            } else {
                format!("pacman -Sy --needed --overwrite \"*\" {}", pkg)
            };
            let excode = Command::new("sh").arg("-c")
                                            .arg(format!("chroot /.snapshots/rootfs/snapshot-chr{} {}", snapshot,pacman_args))
                                            .status()?;
            if !excode.success() {
                return Err(Error::new(ErrorKind::Other,
                                      format!("Failed to install {}", pkg)));
            }
        } else if aur_check(snapshot) {
            // Use paru if aur is enabled
            let paru_args = if noconfirm {
                format!("paru -Sy --noconfirm --needed --overwrite \"*\" {}", pkg)
            } else {
                format!("paru -Sy --needed --overwrite \"*\" {}", pkg)
            };
            let excode = Command::new("chroot")
                .arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                .args(["su", "aur", "-c"])
                .arg(&paru_args)
                .status()?;
            if !excode.success() {
                return Err(Error::new(ErrorKind::Other,
                                      format!("Failed to install {}", pkg)));
            }
        } else if !aur_check(snapshot) {
            return Err(Error::new(ErrorKind::NotFound,
                                  "Please enable AUR."));
        }
    }
    Ok(())
}

// Install atomic-operation in live snapshot
pub fn install_package_helper_live(snapshot: &str, tmp: &str, pkgs: &Vec<String>, noconfirm: bool) -> Result<(), Error> {
    for pkg in pkgs {
        // This extra pacman check is to avoid unwantedly triggering AUR if package is official
        let excode = Command::new("pacman").arg("-Si")
                                           .arg(format!("{}", pkg))
                                           .output()?; // --sysroot
        let excode_group = Command::new("pacman").arg("-Sg")
                                                 .arg(format!("{}", pkg))
                                                 .output()?;
        if excode.status.success() || excode_group.status.success() {
            let pacman_args = if noconfirm {
                format!("pacman -Sy --noconfirm --overwrite '*' {}", pkg)
            } else {
                format!("pacman -Sy --overwrite '*' {}", pkg)
            };
            let excode = Command::new("sh")
                .arg("-c")
                .arg(format!("chroot /.snapshots/rootfs/snapshot-{} {}", tmp,pacman_args))
                .status()?;
            if !excode.success() {
                return Err(Error::new(ErrorKind::Other,
                                      format!("Failed to install {}", pkg)));
            }
        } else if aur_check(snapshot) {
            // Use paru if aur is enabled
            let paru_args = if noconfirm {
                format!("paru -Sy --noconfirm --overwrite '*' {}", pkg)
            } else {
                format!("paru -Sy --overwrite '*' {}", pkg)
            };
            let excode = Command::new("chroot")
                .arg(format!("/.snapshots/rootfs/snapshot-{}", tmp))
                .args(["su", "aur", "-c"])
                .arg(&paru_args)
                .status()?;
            if !excode.success() {
                return Err(Error::new(ErrorKind::Other,
                                      format!("Failed to install {}", pkg)));
            }
        }
    }
    Ok(())
}

// Check if service enabled
pub fn is_service_enabled(snapshot: &str, service: &str) -> bool {
    if Path::new("/var/lib/systemd/").try_exists().unwrap() {
        let excode = Command::new("sh").arg("-c")
                                       .arg(format!("chroot /.snapshots/rootfs/snapshot-chr{} systemctl is-enabled {}", snapshot,service))
                                       .output().unwrap();
        let stdout = String::from_utf8_lossy(&excode.stdout).trim().to_string();
        if stdout == "enabled" {
            return true;
        } else {
            return false;
        }
    } else {
        // TODO add OpenRC
        return false;
    }
}

// Prevent system packages from being automatically removed
pub fn lockpkg(snapshot:&str, profconf: &Ini) -> Result<(), Error> {
    // Open the file
    let pacman_conf_path = format!("/.snapshots/rootfs/snapshot-chr{}/etc/pacman.conf", snapshot);
    let pfile = File::open(&pacman_conf_path)?;
    let reader = BufReader::new(pfile);

    // Get old value for HoldPkg
    let mut old_hold_pkgs = String::new();
    for line in reader.lines() {
        let line = line?;
        if line.starts_with("HoldPkg") {
            old_hold_pkgs.push_str(&line);
            break;
        } else if line.starts_with("#HoldPkg") {
            old_hold_pkgs.push_str(&line);
            break;
        }
    }

    // Replace HoldPkg value
    let mut system_pkgs: Vec<String> = Vec::new();
    if profconf.sections().contains(&"system-packages".to_string()) {
        for pkg in profconf.get_map().unwrap().get("system-packages").unwrap().keys() {
            system_pkgs.push(pkg.to_string());
        }
    }

    if !system_pkgs.is_empty() {
        let pkgs = system_pkgs.iter().map(|s| s.to_string()).collect::<Vec<String>>().join(" ");
        let new_hold_pkgs = format!("HoldPkg = {}", pkgs);
        let mut contents = String::new();
        let mut pfile = File::open(&pacman_conf_path)?;
        pfile.read_to_string(&mut contents)?;
        let modified_pacman_contents = contents.replace(&old_hold_pkgs, &new_hold_pkgs);
        let mut nfile = File::create(&pacman_conf_path)?;
        nfile.write_all(modified_pacman_contents.as_bytes())?;
    }
    Ok(())
}

// Get list of installed packages and exclude packages installed as dependencies
pub fn no_dep_pkg_list(snapshot: &str, chr: &str) -> Vec<String> {
    let excode = Command::new("sh").arg("-c")
                                   .arg(format!("chroot /.snapshots/rootfs/snapshot-{}{} pacman -Qqe", chr,snapshot))
                                   .output().unwrap();
    let stdout = String::from_utf8_lossy(&excode.stdout).trim().to_string();
    stdout.split('\n').map(|s| s.to_string()).collect()
}

// Get list of packages installed in a snapshot
pub fn pkg_list(snapshot: &str, chr: &str) -> Vec<String> {
    prepare(snapshot).unwrap();
    let excode = Command::new("sh").arg("-c")
                                   .arg(format!("chroot /.snapshots/rootfs/snapshot-{}{} pacman -Qq", chr,snapshot))
                                   .output().unwrap();
    post_transactions(snapshot).unwrap();
    let stdout = String::from_utf8_lossy(&excode.stdout).trim().to_string();
    stdout.split('\n').map(|s| s.to_string()).collect()
}

// Pacman query
pub fn pkg_query(pkg: &str) -> Result<ExitStatus, Error> {
    let excode = Command::new("dpkg-query").arg("-W").arg("-f='${Package} ${Version}'").arg(pkg).status();
    excode
}

// Refresh snapshot atomic-operation
pub fn refresh_helper(snapshot: &str) -> Result<(), Error> {
    let refresh = Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                                        .args(["pacman", "-Syy"])
                                        .status()?;
    // Avoid invalid or corrupted package (PGP signature) error
    let keyring = Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                                        .args(["pacman", "-S", "--noconfirm", "archlinux-keyring"])
                                        .status()?;
    if !refresh.success() {
        return Err(Error::new(ErrorKind::Other,
                              "Refresh failed."));
    }
    if !keyring.success() {
        return Err(Error::new(ErrorKind::Other,
                              "Failed to update archlinux-keyring."));
    }
   Ok(())
}

// Disable service(s) (Systemd, OpenRC, etc.)
pub fn service_disable(snapshot: &str, services: &Vec<String>, chr: &str) -> Result<(), Error> {
    for service in services {
        if is_service_enabled(snapshot, service) {
            // Systemd
            if Path::new("/var/lib/systemd/").try_exists()? {
                let excode = Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-{}{}", chr,snapshot))
                                                   .arg("systemctl")
                                                   .arg("disable")
                                                   .arg(&service).status()?;
                if !excode.success() {
                    return Err(Error::new(ErrorKind::Other,
                                          format!("Failed to disable {}.", service)));
                }
            } //TODO add OpenRC
        }
    }
    Ok(())
}

// Enable service(s) (Systemd, OpenRC, etc.)
pub fn service_enable(snapshot: &str, services: &Vec<String>, chr: &str) -> Result<(), Error> {
    for service in services {
        // Systemd
        if Path::new("/var/lib/systemd/").try_exists()? {
            let excode = Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-{}{}", chr,snapshot))
                                               .arg("systemctl")
                                               .arg("enable")
                                               .arg(&service).status()?;
            if !excode.success() {
                return Err(Error::new(ErrorKind::Other,
                                      format!("Failed to enable {}.", service)));
            }
        } //TODO add OpenRC
    }
    Ok(())
}

// Copy system configurations to new snapshot
pub fn system_config(snapshot: &str, profconf: &Ini) -> Result<(), Error> {
    // Copy [fstab, time ,localization, network configuration, users and groups, pacman.conf]
    let files = vec!["/etc/fstab", "/etc/localtime", "/etc/adjtime", "/etc/locale.gen", "/etc/locale.conf",
                     "/etc/vconsole.conf", "/etc/hostname", "/etc/shadow", "/etc/passwd", "/etc/gshadow",
                     "/etc/group", "/etc/sudoers", "/etc/pacman.conf"];

    for file in files {
        if Path::new(&format!("/.snapshots/rootfs/snapshot-{}{}", snapshot,file)).is_file() {
            Command::new("cp").args(["-r", "--reflink=auto"])
                              .arg(format!("/.snapshots/rootfs/snapshot-{}{}", snapshot,file))
                              .arg(format!("/.snapshots/rootfs/snapshot-chr{}{}", snapshot,file)).status()?;
        }
    }

    // Copy pacman.d directory
    remove_dir_content(&format!("/.snapshots/rootfs/snapshot-chr{}/etc/pacman.d", snapshot))?;
    Command::new("cp").args(["-n", "-r", "--reflink=auto"])
                      .arg(format!("/.snapshots/rootfs/snapshot-{}/etc/pacman.d/.", snapshot))
                      .arg(format!("/.snapshots/rootfs/snapshot-chr{}/etc/pacman.d/", snapshot))
                      .output()?;

    // Copy ash configuration
    Command::new("cp").args(["-n", "-r", "--reflink=auto"])
                      .arg(format!("/.snapshots/rootfs/snapshot-{}/etc/ash", snapshot))
                      .arg(format!("/.snapshots/rootfs/snapshot-chr{}/etc/ash", snapshot))
                      .output()?;

    // Copy grub configuration
    #[cfg(feature = "grub")]
    let grub = grub::get_grub(snapshot).unwrap();
    #[cfg(feature = "grub")]
    Command::new("cp").args(["-n", "-r", "--reflink=auto"])
                      .arg(format!("/.snapshots/rootfs/snapshot-{}/boot/{}/.", snapshot,grub))
                      .arg(format!("/.snapshots/rootfs/snapshot-chr{}/boot/{}", snapshot,grub))
                      .output()?;
    #[cfg(feature = "grub")]
    Command::new("cp").args(["-n", "-r", "--reflink=auto"])
                      .arg(format!("/.snapshots/rootfs/snapshot-{}/etc/default/grub", snapshot))
                      .arg(format!("/.snapshots/rootfs/snapshot-chr{}/etc/default/grub", snapshot))
                      .output()?;
    #[cfg(feature = "grub")]
    Command::new("cp").args(["-n", "-r", "--reflink=auto"])
                      .arg(format!("/.snapshots/rootfs/snapshot-{}/etc/grub.d", snapshot))
                      .arg(format!("/.snapshots/rootfs/snapshot-chr{}/etc/grub.d", snapshot))
                      .output()?;

    // Copy /usr/local
    remove_dir_content(&format!("/.snapshots/rootfs/snapshot-chr{}/usr/local", snapshot))?;
    Command::new("cp").args(["-n", "-r", "--reflink=auto"])
                      .arg(format!("/.snapshots/rootfs/snapshot-{}/usr/local/.", snapshot))
                      .arg(format!("/.snapshots/rootfs/snapshot-chr{}/usr/local/", snapshot))
                      .output()?;

    // Install system packages
    if profconf.sections().contains(&"system-packages".to_string()) {
        let mut pkgs_list: Vec<String> = Vec::new();
        for pkg in profconf.get_map().unwrap().get("system-packages").unwrap().keys() {
            pkgs_list.push(pkg.to_string());
        }
        if !pkgs_list.is_empty() {
            install_package_helper_chroot(snapshot, &pkgs_list,true)?;
        }
    }

    if profconf.sections().contains(&"profile-packages".to_string()) {
        let mut pkgs_list: Vec<String> = Vec::new();
        for pkg in profconf.get_map().unwrap().get("profile-packages").unwrap().keys() {
            pkgs_list.push(pkg.to_string());
        }
        if !pkgs_list.is_empty() {
            install_package_helper_chroot(snapshot, &pkgs_list,true)?;
        }
    }

    // Read disable services section in configuration file
    if profconf.sections().contains(&"disable-services".to_string()) {
        let mut services: Vec<String> = Vec::new();
        for service in profconf.get_map().unwrap().get("disable-services").unwrap().keys() {
            services.push(service.to_string());
        }
        // Disable service(s)
        if !services.is_empty() {
            service_disable(snapshot, &services, "chr")?;
        }
    }

    // Read enable services section in configuration file
    if profconf.sections().contains(&"enable-services".to_string()) {
        let mut services: Vec<String> = Vec::new();
        for service in profconf.get_map().unwrap().get("enable-services").unwrap().keys() {
            services.push(service.to_string());
        }
        // Enable service(s)
        if !services.is_empty() {
            service_enable(snapshot, &services, "chr")?;
        }
    }

    // Read commands section in configuration file
    if profconf.sections().contains(&"install-commands".to_string()) {
        for cmd in profconf.get_map().unwrap().get("install-commands").unwrap().keys() {
            chroot_exec(&format!("/.snapshots/rootfs/snapshot-chr{}", snapshot), cmd)?;
        }
    }

    // Restore system configuration
    if profconf.sections().contains(&"system-configuration".to_string()) {
        let mut system_conf: Vec<String> = Vec::new();
        for path in profconf.get_map().unwrap().get("system-configuration").unwrap().keys() {
            // Check if a file or directory exists
            if !metadata(path).is_ok() {
                system_conf.push(path.to_string());
            }
        }
        if !system_conf.is_empty() {
            for path in system_conf {
                Command::new("cp").args(["-r", "--reflink=auto"])
                                  .arg(format!("/.snapshots/rootfs/snapshot-{}{}", snapshot,path))
                                  .arg(format!("/.snapshots/rootfs/snapshot-chr{}{}", snapshot,path)).status()?;
            }
        }
    }

    Ok(())
}

// Sync tree helper function
pub fn tree_sync_helper(s_f: &str, s_t: &str, chr: &str) -> Result<(), Error>  {
    Command::new("cp").args(["-r", "--reflink=auto"])
                      .arg(format!("/.snapshots/etc/etc-{}/.", s_f))
                      .arg(format!("/.snapshots/rootfs/snapshot-{}{}/etc", chr,s_t))
                      .output()?;
    Command::new("cp").args(["-n", "-r", "--reflink=auto"])
                      .arg(format!("/.snapshots/rootfs/snapshot-{}/var/cache/pacman/pkg/.", s_f))
                      .arg(format!("/.snapshots/rootfs/snapshot-{}{}/var/cache/pacman/pkg", chr,s_t))
                      .output()?;
    check_profile(s_t)?;
    Ok(())
}

// Uninstall package(s) atomic-operation
pub fn uninstall_package_helper(snapshot: &str, pkgs: &Vec<String>, noconfirm: bool) -> Result<(), Error> {
    // Profile configurations
    let cfile = format!("/.snapshots/rootfs/snapshot-chr{}/etc/ash/profile", snapshot);
    let mut profconf = Ini::new_cs();
    profconf.set_comment_symbols(&['#']);
    profconf.set_multiline(true);
    let mut write_options = WriteOptions::default();
    write_options.blank_lines_between_sections = 1;
    // Load profile
    profconf.load(&cfile).unwrap();

    for pkg in pkgs {
        let mut pkgs_list: Vec<String> = Vec::new();
        if profconf.sections().contains(&"profile-packages".to_string()) {
            for pkg in profconf.get_map().unwrap().get("profile-packages").unwrap().keys() {
                pkgs_list.push(pkg.to_string());
            }
        }
        let pacman_args = if noconfirm {
            ["pacman", "--noconfirm", "-Rns"]
        } else {
            ["pacman", "--confirm", "-Rns"]
        };

        if !is_system_locked() || !is_system_pkg(&profconf, pkg.to_string()) {
            let excode = Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                                               .args(pacman_args)
                                               .arg(format!("{}", pkg)).status()?;

            if !excode.success() {
            return Err(Error::new(ErrorKind::Other,
                                  format!("Failed to uninstall {}", pkg)));
            } else if pkgs_list.contains(pkg) {
                profconf.remove_key("profile-packages", &pkg);
                profconf.pretty_write(&cfile, &write_options)?;
            } else if is_system_pkg(&profconf, pkg.to_string()) {
                profconf.remove_key("system-packages", &pkg);
                profconf.pretty_write(&cfile, &write_options)?;
            }
        } else if is_system_locked() && is_system_pkg(&profconf, pkg.to_string()){
            return Err(Error::new(ErrorKind::Unsupported,
                                  "Remove system package(s) is not allowed."));
        }
    }
    Ok(())
}

// Uninstall package(s) atomic-operation
pub fn uninstall_package_helper_chroot(snapshot: &str, pkgs: &Vec<String>, noconfirm: bool) -> Result<(), Error> {
    for pkg in pkgs {
        let pacman_args = if noconfirm {
            ["pacman", "--noconfirm", "-Rns"]
        } else {
            ["pacman", "--confirm", "-Rns"]
        };

        let excode = Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                                           .args(pacman_args)
                                           .arg(format!("{}", pkg)).status()?;

        if !excode.success() {
            return Err(Error::new(ErrorKind::Other,
                                  format!("Failed to uninstall {}", pkg)));
        }
    }
    Ok(())
}

// Uninstall package(s) atomic-operation live snapshot
pub fn uninstall_package_helper_live(tmp: &str, pkgs: &Vec<String>, noconfirm: bool) -> Result<(), Error> {
    for pkg in pkgs {
        let pacman_args = if noconfirm {
            ["pacman", "--noconfirm", "-Rns"]
        } else {
            ["pacman", "--confirm", "-Rns"]
        };

        let excode = Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-{}", tmp))
                                           .args(pacman_args)
                                           .arg(format!("{}", pkg)).status()?;

        if !excode.success() {
            return Err(Error::new(ErrorKind::Other,
                                  format!("Failed to uninstall {}", pkg)));
        }
    }
    Ok(())
}

// Upgrade snapshot atomic-operation
pub fn upgrade_helper(snapshot: &str, noconfirm: bool) -> Result<(), Error> {
    // Prepare snapshot
    prepare(snapshot).unwrap();
    // Avoid invalid or corrupted package (PGP signature) error
    let pacman_args = if noconfirm {
        ["pacman", "--noconfirm", "-Syy", "archlinux-keyring"]
    } else {
        ["pacman", "--confirm", "-Syy", "archlinux-keyring"]
    };

    Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                          .args(pacman_args)
                          .status().unwrap();
    if !aur_check(snapshot) {
        let pacman_args = if noconfirm {
            ["pacman", "--noconfirm", "-Syyu"]
        } else {
            ["pacman", "--confirm", "-Syyu"]
        };

        let excode = Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                                            .args(pacman_args)
                                            .status().unwrap();
        if !excode.success() {
            return Err(Error::new(ErrorKind::Other,
                                  format!("Failed to upgrade snapshot {}.", snapshot)));
        }
    } else {
        let paru_args = if noconfirm {
            "paru --noconfirm -Syyu"
        } else {
            "paru -Syyu"
        };

        let excode = Command::new("sh").arg("-c")
                                        .arg(format!("chroot /.snapshots/rootfs/snapshot-chr{} su aur -c '{}'", snapshot,paru_args))
                                        .status().unwrap();
        if !excode.success() {
            return Err(Error::new(ErrorKind::Other,
                                  format!("Failed to upgrade snapshot {}.", snapshot)));
        }
    }
    Ok(())
}

// Live upgrade snapshot atomic-operation
pub fn upgrade_helper_live(tmp: &str, noconfirm: bool) -> Result<(), Error> {
    // Avoid invalid or corrupted package (PGP signature) error
    let pacman_args = if noconfirm {
        ["pacman", "--noconfirm", "-Syy", "archlinux-keyring"]
    } else {
        ["pacman", "--confirm", "-Syy", "archlinux-keyring"]
    };

    Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-{}", tmp))
                          .args(pacman_args)
                          .status().unwrap();
    if !aur_check(tmp) {
        let pacman_args = if noconfirm {
            ["pacman", "--noconfirm", "-Syyu"]
        } else {
            ["pacman", "--confirm", "-Syyu"]
        };

        let excode = Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-{}", tmp))
                                           .args(pacman_args)
                                           .status().unwrap();
        if !excode.success() {
            return Err(Error::new(ErrorKind::Other,
                                  "Failed to upgrade current/live snapshot."));
        }
    } else {
        let paru_args = if noconfirm {
            "paru --noconfirm -Syyu"
        } else {
            "paru --confirm -Syyu"
        };

        let excode = Command::new("sh")
            .arg("-c")
            .arg(format!("chroot /.snapshots/rootfs/snapshot-{} su aur -c '{}'",
                         tmp,paru_args))
            .status().unwrap();
        if !excode.success() {
            return Err(Error::new(ErrorKind::Other,
                                  "Failed to upgrade current/live snapshot."));
        }
    }
    Ok(())
}