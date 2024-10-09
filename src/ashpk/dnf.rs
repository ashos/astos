use crate::{check_profile, chr_delete, chroot_exec, get_tmp, grub, is_system_pkg,
            is_system_locked, post_transactions, prepare,
            remove_dir_content, sync_time};

use configparser::ini::{Ini, WriteOptions};
use rustix::path::Arg;
use std::fs::{metadata, OpenOptions, read_to_string};
use std::io::{Error, ErrorKind, Write};
use std::path::Path;
use std::process::{Command, ExitStatus};

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

        // Use dnf
        let dnf_upgrade = "dnf -y upgrade --refresh";
        let excode = Command::new("sh").arg("-c")
                                       .arg(format!("chroot /.snapshots/rootfs/snapshot-chr{} {}", snapshot,dnf_upgrade))
                                       .status()?;
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
    Ok(())
}

// Reinstall base packages in snapshot
pub fn bootstrap(snapshot: &str) -> Result<(), Error> {
    let packages = "basesystem dnf glibc-all-langpacks";
    let release = code_name(snapshot);
    let target_path = format!("/.snapshots/rootfs/snapshot-chr{}", snapshot);
    let bootstrap_cmd = format!("dnf --installroot='{}' install -y {} --releasever='{}'", target_path,packages,release);

    // Bootstrap command
    let excode = Command::new("sh").arg("-c").arg(bootstrap_cmd).status()?;

    if !excode.success() {
        return Err(Error::new(ErrorKind::Other,
                              "Failed to bootstrap."));
    }
    Ok(())
}

// Copy cache of downloaded packages to shared
pub fn cache_copy(snapshot: &str, prepare: bool) -> Result<(), Error> {
    let tmp = get_tmp();
    if prepare {
        Command::new("cp").args(["-n", "-r", "--reflink=auto"])
                          .arg(format!("/.snapshots/rootfs/snapshot-{}/var/cache/dnf/.", snapshot))
                          .arg(format!("/.snapshots/rootfs/snapshot-chr{}/var/cache/dnf", tmp))
                          .output().unwrap();
    } else {
        Command::new("cp").args(["-n", "-r", "--reflink=auto"])
                          .arg(format!("/.snapshots/rootfs/snapshot-chr{}/var/cache/dnf/.", snapshot))
                          .arg(format!("/.snapshots/rootfs/snapshot-{}/var/cache/dnf", tmp))
                          .output().unwrap();
    }
    Ok(())
}

// Clean dnf cache
pub fn clean_cache(snapshot: &str) -> Result<(), Error> {
    if Path::new(&format!("/.snapshots/rootfs/snapshot-chr{}/var/cache/dnf", snapshot)).try_exists().unwrap() {
        remove_dir_content(&format!("/.snapshots/rootfs/snapshot-chr{}/var/cache/dnf", snapshot))?;
    }
    Ok(())
}

// Distribution code name
pub fn code_name(snapshot: &str) -> String {
    let mut code_name = String::new();
    // Check if /etc/os-release exists and contains ID
    if code_name.is_empty() {
        if let Ok(file) = read_to_string(&format!("/.snapshots/rootfs/snapshot-{}/etc/os-release", snapshot)) {
            for line in file.lines() {
                if line.starts_with("VERSION_ID") {
                    code_name = line.split('=').nth(1).unwrap()
                                                      .to_lowercase()
                                                      .trim_matches('"')
                                                      .to_string();
                    break;
                }
            }
        }
    }

    code_name
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
        // Nocomfirm flag
        let install_args = if noconfirm {
            format!("dnf install -y --setopt install_weak_deps=False --refresh {}", pkg)
        } else {
            format!("dnf install --setopt install_weak_deps=False --refresh {}", pkg)
        };

        // Install packages using dnf
        let dnf_install = Command::new("sh").arg("-c")
                                       .arg(format!("chroot /.snapshots/rootfs/snapshot-chr{} {}", snapshot,&install_args))
                                       .status()?;
        if !dnf_install.success() {
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
    }
    Ok(())
}

// Install atomic-operation
pub fn install_package_helper_chroot(snapshot:&str, pkgs: &Vec<String>, noconfirm: bool) -> Result<(), Error> {
    let pkg_list = format!("{pkgs:?}");
    let install_args = if noconfirm {
        format!("dnf install -y --setopt install_weak_deps=False --refresh {}", pkg_list.replace(&['[', ']', ',', '\"'][..], ""))
    } else {
        format!("dnf install --setopt install_weak_deps=False --refresh {}", pkg_list.replace(&['[', ']', ',', '\"'][..], ""))
    };

    // DNF install
    let dnf_install = Command::new("sh").arg("-c")
                                        .arg(format!("chroot /.snapshots/rootfs/snapshot-chr{} {}", snapshot,install_args))
                                        .status()?;

    if !dnf_install.success() {
        return Err(Error::new(ErrorKind::Other,
                              "Failed to install package(s)."));
    }
    Ok(())
}

// Install atomic-operation in live snapshot
pub fn install_package_helper_live(_snapshot: &str, tmp: &str, pkgs: &Vec<String>, noconfirm: bool) -> Result<(), Error> {
    for pkg in pkgs {
        let install_args = if noconfirm {
            format!("dnf install -y --setopt install_weak_deps=False --refresh {}", pkg)
        } else {
            format!("dnf install --setopt install_weak_deps=False --refresh {}", pkg)
        };

        // DNF install
        let dnf_install = Command::new("sh").arg("-c")
                                            .arg(format!("chroot /.snapshots/rootfs/snapshot-{} {}", tmp,install_args))
                                            .status()?;

        if !dnf_install.success() {
            return Err(Error::new(ErrorKind::Other,
                                  format!("Failed to install {}", pkg)));
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
    let mut system_pkgs: Vec<String> = Vec::new();
    if profconf.sections().contains(&"system-packages".to_string()) {
        for pkg in profconf.get_map().unwrap().get("system-packages").unwrap().keys() {
            system_pkgs.push(pkg.to_string());
        }
    }

    let mut lockpkg = String::new();
    if !system_pkgs.is_empty() {
        for pkg in system_pkgs {
            let rule = format!("{}\n", pkg);
            lockpkg.push_str(&rule);
        }
    }
    let mut rule_file = OpenOptions::new().truncate(true)
                                          .create(true)
                                          .read(true)
                                          .write(true)
                                          .open(format!("/.snapshots/rootfs/snapshot-chr{}/etc/dnf/protected.d/ash_system_packages.conf", snapshot))?;
    rule_file.write_all(lockpkg.as_bytes())?;
    Ok(())
}

// Get list of installed packages and exclude packages installed as dependencies
pub fn no_dep_pkg_list(snapshot: &str, chr: &str) -> Vec<String> {
    let dpkg_query = "dnf repoquery -C --userinstalled --qf='%{name}'";
    let excode = Command::new("sh").arg("-c")
                                   .arg(format!("chroot /.snapshots/rootfs/snapshot-{}{} {}", chr,snapshot,dpkg_query))
                                   .output().unwrap();
    let stdout = String::from_utf8_lossy(&excode.stdout).trim().to_string();
    stdout.split('\n').map(|s| s.to_string()).collect()
}

// Get list of packages installed in a snapshot
pub fn pkg_list(snapshot: &str, chr: &str) -> Vec<String> {
    prepare(snapshot).unwrap();
    let dpkg_query = "dnf repoquery -C --installed --qf='%{name}'";
    let excode = Command::new("sh").arg("-c")
                                   .arg(format!("chroot /.snapshots/rootfs/snapshot-{}{} {}", chr,snapshot,dpkg_query))
                                   .output().unwrap();
    post_transactions(snapshot).unwrap();
    let stdout = String::from_utf8_lossy(&excode.stdout).trim().to_string();
    stdout.split('\n').map(|s| s.to_string()).collect()
}

// DNF query
pub fn pkg_query(pkg: &str) -> Result<ExitStatus, Error> {
    let rpm_query = "rpm -q --queryformat '%{NAME} %{VERSION}\n'";
    let excode = Command::new("sh").arg("-c")
                                   .arg(format!("{} {}", rpm_query,pkg))
                                   .status();
    excode
}

// Refresh snapshot atomic-operation
pub fn refresh_helper(snapshot: &str) -> Result<(), Error> {
    let refresh = Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                                        .args(["dnf", "update"])
                                        .status()?;
    if !refresh.success() {
        return Err(Error::new(ErrorKind::Other,
                              "Refresh failed."));
    }
    Ok(())
}

// Disable service(s) (Systemd, OpenRC, etc.)
pub fn service_disable(snapshot: &str, services: &Vec<String>, chr: &str) -> Result<(), Error> {
    for service in services {
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
        } //TODO add OspenRC
    }
    Ok(())
}

// Copy system configurations to new snapshot
pub fn system_config(snapshot: &str, profconf: &Ini) -> Result<(), Error> {
    // Copy [fstab, time ,localization, network configuration, users and groups]
    let files = vec!["/etc/fstab", "/etc/localtime", "/etc/adjtime", "/etc/locale.gen", "/etc/locale.conf",
                     "/etc/vconsole.conf", "/etc/hostname", "/etc/shadow", "/etc/passwd", "/etc/gshadow",
                     "/etc/group", "/etc/sudoers"];
    for file in files {
        if Path::new(&format!("/.snapshots/rootfs/snapshot-{}{}", snapshot,file)).is_file() {
            Command::new("cp").args(["-r", "--reflink=auto"])
                              .arg(format!("/.snapshots/rootfs/snapshot-{}{}", snapshot,file))
                              .arg(format!("/.snapshots/rootfs/snapshot-chr{}{}", snapshot,file)).status()?;
        }
    }

    // Copy dnf configuration
    remove_dir_content(&format!("/.snapshots/rootfs/snapshot-chr{}/etc/dnf", snapshot))?;
    Command::new("cp").args(["-n", "-r", "--reflink=auto"])
                      .arg(format!("/.snapshots/rootfs/snapshot-{}/etc/dnf/.", snapshot))
                      .arg(format!("/.snapshots/rootfs/snapshot-chr{}/etc/dnf/", snapshot))
                      .output()?;
    remove_dir_content(&format!("/.snapshots/rootfs/snapshot-chr{}/etc/yum.repos.d", snapshot))?;
    Command::new("cp").args(["-n", "-r", "--reflink=auto"])
                      .arg(format!("/.snapshots/rootfs/snapshot-{}/etc/yum.repos.d/.", snapshot))
                      .arg(format!("/.snapshots/rootfs/snapshot-chr{}/etc/yum.repos.d/", snapshot))
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

    // Install system packages
    if profconf.sections().contains(&"system-packages".to_string()) {
        let mut pkgs_list: Vec<String> = Vec::new();
        for pkg in profconf.get_map_ref().get("system-packages").unwrap().keys() {
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
                      .arg(format!("/.snapshots/rootfs/snapshot-{}/var/cache/dnf/.", s_f))
                      .arg(format!("/.snapshots/rootfs/snapshot-{}{}/var/cache/dnf", chr,s_t))
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
        let uninstall_args = if noconfirm {
            ["dnf", "remove", "-y"]
        } else {
            ["dnf", "remove", ""]
        };

        if !is_system_locked() || !is_system_pkg(&profconf, pkg.to_string()) {
            let excode = Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-chr{}", snapshot))
                                               .args(uninstall_args)
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
        let uninstall_args = if noconfirm {
            "dnf remove -y"
        } else {
            "dnf remove"
        };

        let excode = Command::new("sh").arg("-c")
                                       .arg(format!("chroot /.snapshots/rootfs/snapshot-chr{} {}", snapshot,uninstall_args))
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
        let uninstall_args = if noconfirm {
            ["dnf", "remove", "-y"]
        } else {
            ["dnf", "remove", ""]
        };

        let excode = Command::new("chroot").arg(format!("/.snapshots/rootfs/snapshot-{}", tmp))
                                           .args(uninstall_args)
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

    let upgrade_args = if noconfirm {
        "dnf upgrade -y --refresh"
    } else {
        "dnf upgrade --refresh"
    };

    // Run dnf upgrade
    let dnf_upgrade = Command::new("sh").arg("-c")
                                        .arg(format!("chroot /.snapshots/rootfs/snapshot-chr{} {}", snapshot,upgrade_args))
                                        .status().unwrap();
    if !dnf_upgrade.success() {
        return Err(Error::new(ErrorKind::Other,
                              format!("Failed to upgrade snapshot {}.", snapshot)));
    }
    Ok(())
}

// Live upgrade snapshot atomic-operation
pub fn upgrade_helper_live(tmp: &str, noconfirm: bool) -> Result<(), Error> {
    let upgrade_args = if noconfirm {
        "dnf upgrade -y --refresh"
    } else {
        "dnf upgrade --refresh"
    };

    // Run dnf upgrade
    let excode = Command::new("sh").arg("-c")
                                   .arg(format!("chroot /.snapshots/rootfs/snapshot-{} {}", tmp,upgrade_args))
                                   .status().unwrap();
    if !excode.success() {
        return Err(Error::new(ErrorKind::Other,
                              "Failed to upgrade current/live snapshot."));
    }
    Ok(())
}