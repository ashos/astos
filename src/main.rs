extern crate lib;
pub mod btrfs;
mod cli;

use cli::*;
use lib::*;
use nix::unistd::Uid;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::Command;

// Select Bootloader
#[cfg(feature = "grub")]
pub mod grub;
#[cfg(feature = "grub")]
use grub::update_boot;
//TODO add systemd-boot


// Directories
// Global boot is always at @boot
// *-chr                             : temporary directories used to chroot into snapshot or copy snapshots around
// *-deploy and *-deploy-aux         : temporary directories used to boot deployed snapshot
// *-deploy[-aux]-secondary          : temporary directories used to boot secondary deployed snapshot
// *-recovery-deploy[-aux]           : temporary directories used to boot deployed recovery snapshot
// /.snapshots/ash/deploy-tmp        : deployed snapshots temporary boot directories
// /.snapshots/ash/ash-{distro_name} : ash binary file location
// /.snapshots/ash/export            : default export path
// /.snapshots/ash/part              : root partition uuid
// /.snapshots/ash/snapshots/*-desc  : snapshots descriptions
// /.snapshots/ash/upstate           : state of last system update
// /.snapshots/boot/boot-*           : individual /boot for each snapshot
// /.snapshots/etc/etc-*             : individual /etc for each snapshot
// /.snapshots/var/var-*             : individual /var for each snapshot
// /.snapshots/rootfs/snapshot-*     : snapshots
// /.snapshots/tmp                   : temporary directory
// /etc/ash/ash.conf                 : configuration file for ash
// /etc/ash/profile                  : snapshot profile
// /usr/sbin/ash                     : symlink to /.snapshots/ash/ash
// /usr/share/ash                    : files that store current snapshot info
// /use/share/ash/profiles           : default desktop environments profiles path
// /use/share/ash/rec-tmp            : name of temporary directory used to boot recovery snapshot
// /use/share/ash/snap               : snapshot number
// /var/lib/ash(/fstree)             : ash files, stores fstree, symlink to /.snapshots/ash/fstree

fn main() {
    if !Uid::effective().is_root() {
        eprintln!("sudo/doas is required to run ash!");
    } else if chroot_check() {
        eprintln!("Please don't use ash inside a chroot!");
    } else {
        // Call cli matches
        let matches = cli().get_matches();
        // Call relevant functions
        match matches.subcommand() {
            // Auto upgrade
            Some(("auto-upgrade", auto_upgrade_matches)) => {
                // Get snapshot value
                let snapshot = if auto_upgrade_matches.contains_id("SNAPSHOT") {
                    let snap = auto_upgrade_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Run noninteractive_update
                let run = noninteractive_update(&snapshot);
                match run {
                    Ok(_) => println!("The auto-upgrade has been completed successfully."),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Base import
            Some(("base-import", base_import_matches)) => {
                // Get user_profile value
                let path: String = base_import_matches.get_many::<String>("SNAPSHOT_PATH").unwrap().map(|s| format!("{}", s)).collect();

                // Get tmp_dir value
                let tmp_dir = tempfile::TempDir::new_in("/.snapshots/tmp").unwrap();

                // Run import_base
                let run = import_base(&path, &tmp_dir);
                if cfg!(feature = "base-import") {
                    match run {
                        Ok(_) => {
                            if post_transactions("0").is_ok() {
                                println!("New base snapshot has been successfully imported.")
                            } else {
                                // Clean chroot mount directories
                                chr_delete("0").unwrap();
                                eprintln!("Failed to import new base snapshot.")
                            }
                        },
                        Err(snapshot) => {
                            // Clean tmp
                            if Path::new(&format!("{}/{}", tmp_dir.path().to_str().unwrap(),snapshot)).try_exists().unwrap() {
                                #[cfg(feature = "btrfs")]
                                btrfs::delete_subvolume(&format!("{}/{}", tmp_dir.path().to_str().unwrap(),snapshot)).unwrap();
                            }
                            // Clean chroot mount directories
                            chr_delete("0").unwrap();
                            eprintln!("{}", snapshot);
                        },
                    }
                } else {
                    eprintln!("base-import subcommand is not supported.");
                }
            }
            // Base rebuild
            Some(("base-rebuild", _matches)) => {
                // Run rebuild_base
                let run = rebuild_base();
                match run {
                    Ok(_) => println!("Base snapshot was rebuilt successfully."),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Base update
            Some(("base-update", base_update_matches)) => {
                // Optional value
                let noconfirm = base_update_matches.get_flag("noconfirm");

                // Run upgrade(0)
                let run = upgrade("0", true, noconfirm);
                match run {
                    Ok(_) => {},
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Boot update command
            Some(("boot", boot_matches)) => {
                // Get snapshot value
                let snapshot = if boot_matches.contains_id("SNAPSHOT") {
                    let snap = boot_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Run update_boot
                let run = update_boot(&snapshot, false);
                match run {
                    Ok(_) => println!("Bootloader updated successfully."),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Branch
            Some(("branch", branch_matches)) => {
                // Get snapshot value
                let snapshot = if branch_matches.contains_id("SNAPSHOT") {
                    let snap = branch_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Get desc value
                let desc = if branch_matches.contains_id("DESCRIPTION") {
                    let desc = branch_matches.get_one::<String>("DESCRIPTION").map(|s| s.as_str()).unwrap().to_string();
                    desc
                } else {
                    let desc = String::new();
                    desc
                };

                // Run barnch_create
                let run = branch_create(&snapshot, &desc);
                match run {
                    Ok(snapshot_num) => println!("Branch {} added under snapshot {}.", snapshot_num,snapshot),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Check update
            Some(("check", _matches)) => {
                // Run check_update
                let run = check_update();
                match run {
                    Ok(_) => (),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Chroot
            Some(("chroot", chroot_matches)) => {
                // Get snapshot value
                let snapshot  = if chroot_matches.contains_id("SNAPSHOT") {
                    let snap = chroot_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Get cmd value
                let cmd: Vec<String> = Vec::new();

                // Run chroot
                let run = chroot(&snapshot, cmd);
                match run {
                    Ok(_) => (),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Clone
            Some(("clone", clone_matches)) => {
                // Get snapshot value
                let snapshot = if clone_matches.contains_id("SNAPSHOT") {
                    let snap = clone_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Get desc value
                let desc = if clone_matches.contains_id("DESCRIPTION") {
                    let desc = clone_matches.get_one::<String>("DESCRIPTION").map(|s| s.as_str()).unwrap().to_string();
                    desc
                } else {
                    let desc = String::new();
                    desc
                };

                // Run clone_as_tree
                let run = clone_as_tree(&snapshot, &desc);
                match run {
                    Ok(snapshot_num) => println!("Tree {} cloned from {}.", snapshot_num,snapshot),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Clone a branch
            Some(("clone-branch", clone_branch_matches)) => {
                // Get snapshot value
                let snapshot = if clone_branch_matches.contains_id("SNAPSHOT") {
                    let snap = clone_branch_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Run clone_branch
                let run = clone_branch(&snapshot);
                match run {
                    Ok(snapshot_num) => println!("Branch {} added to parent of {}.", snapshot_num,snapshot),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Clone recursively
            Some(("clone-tree", clone_tree_matches)) => {
                // Get snapshot value
                let snapshot = if clone_tree_matches.contains_id("SNAPSHOT") {
                    let snap = clone_tree_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Run clone_recursive
                let run = clone_recursive(&snapshot);
                match run {
                    Ok(_) => println!("Snapshot {} was cloned recursively.", snapshot),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Clone under a branch
            Some(("clone-under", clone_under_matches)) => {
                // Get snapshot value
                let snap = clone_under_matches.get_one::<i32>("SNAPSHOT").unwrap();
                let snapshot = format!("{}", snap);

                // Get branch value
                let branch_i32 = clone_under_matches.get_one::<i32>("BRANCH").unwrap();
                let branch = format!("{}", branch_i32);

                // Run clone_under
                let run = clone_under(&snapshot, &branch);
                match run {
                    Ok(snapshot_num) => println!("Branch {} added to parent of {}.", snapshot_num,snapshot),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Current snapshot
            Some(("current", _matches)) => {
                // Run get_current_snapshot
                println!("{}", get_current_snapshot());
            }
            // Delete
            Some(("del", del_matches)) => {
                // Get snapshot value
                let snapshots: Vec<_> = del_matches.get_many::<i32>("SNAPSHOT").unwrap().map(|s| format!("{}", s)).collect();

                // Optional values
                let quiet = del_matches.get_flag("quiet");
                let nuke = del_matches.get_flag("nuke");

                // Run delelte_node
                let run = delete_node(&snapshots, quiet, nuke);
                match run {
                    Ok(_) => println!("Snapshot(s) {:?} removed.", snapshots),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Deploy
            Some(("deploy", deploy_matches)) => {
                // Get snapshot value
                let snapshot = if deploy_matches.contains_id("SNAPSHOT") {
                    let snap = deploy_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Optional value
                let secondary = deploy_matches.get_flag("secondary");

                // Run deploy
                let run = deploy(&snapshot, secondary, false);
                match run {
                    Ok(target_dep) => {
                        // Save deployed snapshots tmp
                        let source_dep = get_tmp();
                        let mut deploy_tmp = OpenOptions::new().truncate(true)
                                                               .create(true)
                                                               .read(true)
                                                               .write(true)
                                                               .open("/.snapshots/ash/deploy-tmp").unwrap();
                        let tmp = format!("{}\n{}", source_dep,target_dep);
                        deploy_tmp.write_all(tmp.as_bytes()).unwrap();
                        println!("Snapshot {} deployed to '/'.", snapshot);
                    },
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Description
            Some(("desc", desc_matches)) => {
                // Get snapshot value
                let snapshot = if desc_matches.contains_id("SNAPSHOT") {
                    let snap = desc_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Get desc value
                let desc = desc_matches.get_one::<String>("DESCRIPTION").map(|s| s.as_str()).unwrap().to_string();

                // Run write_desc
                let run = write_desc(&snapshot, &desc, true);
                match run {
                    Ok(_) => (),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Diff two snapshots
            Some(("diff", diff_matches)) => {
                // Get snapshot one value
                let snap1 = diff_matches.get_one::<i32>("SNAPSHOT-1").unwrap();
                let snapshot1 = format!("{}", snap1);

                // Get snapshot two value
                let snap2 = diff_matches.get_one::<i32>("SNAPSHOT-2").unwrap();
                let snapshot2 = format!("{}", snap2);

                // Run diff
                diff(&snapshot1, &snapshot2);
            }
            // Edit Ash configuration
            Some(("edit", edit_matches)) => {
                // Get snapshot value
                let snapshot = if edit_matches.contains_id("SNAPSHOT") {
                    let snap = edit_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Run snapshot_config_edit
                let run = snapshot_config_edit(&snapshot);
                match run {
                    Ok(_) => (),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Edit snapshot profile
            Some(("edit-profile", edit_profile_matches)) => {
                // Get snapshot value
                let snapshot = if edit_profile_matches.contains_id("SNAPSHOT") {
                    let snap = edit_profile_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Run snapshot_profile_edit
                let run = snapshot_profile_edit(&snapshot);
                match run {
                    Ok(_) => {
                        if post_transactions(&snapshot).is_ok() {
                            println!("snapshot {} profile has been successfully updated.", &snapshot)
                        } else {
                            chr_delete(&snapshot).unwrap();
                            eprintln!("Failed to update snapshot {} profile.", &snapshot)
                        }
                    },
                    Err(e) => {
                        chr_delete(&snapshot).unwrap();
                        eprintln!("{}", e);
                    },
                }
            }
            // Switch distros
            Some(("efi-update", _matches)) => { //REVIEW
                // Run efi_boot_order
                if is_efi() {
                    efi_boot_order().unwrap();
                } else {
                   eprintln!("efi-update command is not supported.");
                }
            }
            // etc update
            Some(("etc-update", _matches)) => {
                // Run update_etc
                let run = update_etc();
                match run {
                    Ok(_) => println!("etc has been successfully updated."),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Export snapshot
            Some(("export", export_matches)) => {
                // Get snapshot value
                let snapshot = if export_matches.contains_id("SNAPSHOT") {
                    let snap = export_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Get dest value
                let dest = if export_matches.contains_id("OUTPUT") {
                    let dest = export_matches.get_one::<String>("OUTPUT").map(|s| s.as_str()).unwrap().to_string();
                    dest
                } else {
                    let dest = String::from("/.snapshots/ash/export");
                    dest
                };

                // Run export
                if !Path::new("/.snapshots/ash/export").try_exists().unwrap() {
                    Command::new("mkdir").arg("-p")
                                         .arg("/.snapshots/ash/export")
                                         .status().unwrap();
                }

                // Run export
                let run = export(&snapshot, &dest);
                match run {
                    Ok(_) => (),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Fix db commands
            Some(("fixdb", fixdb_matches)) => {
                // Get snapshot value
                let snapshot = if fixdb_matches.contains_id("SNAPSHOT") {
                    let snap = fixdb_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = String::new();
                    snap
                };

                //Run fixdb
                let run = fixdb(&snapshot);
                if cfg!(feature = "pacman") {
                    match run {
                        Ok(_) => {
                            let snapshot = if snapshot.is_empty() {
                                get_current_snapshot().to_string()
                            } else {
                                snapshot.clone()
                            };
                            if post_transactions(&snapshot).is_ok() {
                                println!("Snapshot {}'s package manager database fixed successfully.", snapshot);
                            } else {
                                eprintln!("Fixing package manager database failed.");
                            }
                        },
                        Err(e) => {
                            let snapshot = if snapshot.is_empty() {
                                get_current_snapshot().to_string()
                            } else {
                                snapshot.clone()
                            };
                            chr_delete(&snapshot).unwrap();
                            eprintln!("{}", e);
                        },
                    }
                } else {
                    eprintln!("fixdb subcommand is not supported.");
                }
            }
            // Switch to Windows (semi plausible deniability)
            Some(("hide", _matches)) => {
                // Run switch_to_windows
                switch_to_windows();
            }
            // Hollow a node
            Some(("hollow", hollow_matches)) => {
                // Get snapshot value
                let snapshot = if hollow_matches.contains_id("SNAPSHOT") {
                    let snap = hollow_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Run hollow
                let run = hollow(&snapshot);
                match run {
                    Ok(_) => println!("Snapshot {} hollow operation succeeded. Please reboot!", snapshot),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Immutability disable
            Some(("immdis", immdis_matches)) => {
                // Get snapshot value
                let snapshot = if immdis_matches.contains_id("SNAPSHOT") {
                    let snap = immdis_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Run immutability_disable
                let run = immutability_disable(&snapshot);
                match run {
                    Ok(_) => println!("Snapshot {} successfully made mutable.", snapshot),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Immutability enable
            Some(("immen", immen_matches)) => {
                // Get snapshot value
                let snapshot = if immen_matches.contains_id("SNAPSHOT") {
                    let snap = immen_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Run immutability_enable
                let run = immutability_enable(&snapshot);
                match run {
                    Ok(_) => println!("Snapshot {} successfully made immutable.", snapshot),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Import snapshot
            Some(("import", import_matches)) => {
                // Get snapshot value
                let snapshot = find_new();

                // Get desc value
                let desc = if import_matches.contains_id("DESCRIPTION") {
                    let desc = import_matches.get_one::<String>("DESCRIPTION").map(|s| s.as_str()).unwrap().to_string();
                    desc
                } else {
                    let desc = String::new();
                    desc
                };

                // Get user_profile value
                let path: String = import_matches.get_many::<String>("SNAPSHOT_PATH").unwrap().map(|s| format!("{}", s)).collect();

                // Get tmp_dir value
                let tmp_dir = tempfile::TempDir::new_in("/.snapshots/tmp").unwrap();

                // Run import
                let run = import(snapshot, &path, &desc, &tmp_dir);
                if cfg!(feature = "import") {
                    match run {
                        Ok(_) => {
                            println!("Snapshot {} has been successfully imported.", snapshot);
                        },
                        Err(e) => {
                            // Clean tmp
                            if Path::new(&format!("{}/{}", tmp_dir.path().to_str().unwrap(),snapshot)).try_exists().unwrap() {
                                #[cfg(feature = "btrfs")]
                                btrfs::delete_subvolume(&format!("{}/{}", tmp_dir.path().to_str().unwrap(),snapshot)).unwrap();
                            }
                            // Clean chroot mount directories
                            chr_delete(&format!("{}", snapshot)).unwrap();
                            eprintln!("{}", e);
                        },
                    }
                } else {
                    eprintln!("import subcommand is not supported.");
                }
            }
            // Install command
            Some(("install", install_matches)) => {
                // Get snapshot value
                let snapshot = if install_matches.contains_id("SNAPSHOT") {
                    let snap = install_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Get pkgs value
                let pkgs = if install_matches.contains_id("PACKAGE") {
                    let pkgs: Vec<String> = install_matches.get_many::<String>("PACKAGE").unwrap().map(|s| format!("{}", s)).collect();
                    pkgs
                } else {
                    let pkgs: Vec<String> = Vec::new();
                    pkgs
                };

                // Get profile value
                let profile = if install_matches.contains_id("PROFILE") {
                    let profile = install_matches.get_many::<String>("PROFILE").unwrap().map(|s| format!("{}", s)).collect();
                    profile
                } else {
                    let profile = String::new();
                    profile
                };

                // Get user_profile value
                let user_profile = if install_matches.contains_id("USER_PROFILE") {
                    let user_profile = install_matches.get_many::<String>("USER_PROFILE").unwrap().map(|s| format!("{}", s)).collect();
                    user_profile
                } else {
                    let user_profile = String::new();
                    user_profile
                };

                // Optional values
                let live = install_matches.get_flag("live");
                let noconfirm = install_matches.get_flag("noconfirm");
                let force = install_matches.get_flag("force");
                let secondary = install_matches.get_flag("secondary");

                // Run install_triage
                install_triage(&snapshot, live, pkgs, &profile, force, &user_profile, noconfirm, secondary).unwrap();
            }
            // Package list
            Some(("list", list_matches)) => {
                // Get snapshot value
                let snapshot = if list_matches.contains_id("SNAPSHOT") {
                    let snap = list_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // chr value
                let chr = "";

                // Optional value
                let exclude = list_matches.get_flag("exclude-dependency");

                // Make sure snapshot exists
                if !Path::new(&format!("/.snapshots/rootfs/snapshot-{}", snapshot)).try_exists().unwrap() {
                    eprintln!("Cannot list packages as snapshot {} doesn't exist.", snapshot);
                } else {
                    // Run list
                    let run = list(&snapshot, chr, exclude);
                    for pkg in run {
                        println!("{}", pkg);
                    }
                }
            }
            // Live chroot
            Some(("live-chroot", _matches)) => {
                // Run live_unlock
                let run = live_unlock();
                match run {
                    Ok(_) => (),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // New
            Some(("new", new_matches)) => {
                // Get desc value
                let desc = if new_matches.contains_id("DESCRIPTION") {
                    let desc = new_matches.get_one::<String>("DESCRIPTION").map(|s| s.as_str()).unwrap().to_string();
                    desc
                } else {
                    let desc = String::new();
                    desc
                };

                // Run snapshot_base_new
                let run = snapshot_base_new(&desc);
                match run {
                    Ok(snap_num) => println!("New tree {} created.", snap_num),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Rebuild
            Some(("rebuild", rebuild_matches)) => {
                // Get snapshot value
                let snapshot = if rebuild_matches.contains_id("SNAPSHOT") {
                    let snap = rebuild_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Get desc value
                let desc = if rebuild_matches.contains_id("DESCRIPTION") {
                    let desc = rebuild_matches.get_one::<String>("DESCRIPTION").map(|s| s.as_str()).unwrap().to_string();
                    desc
                } else {
                    let desc = String::new();
                    desc
                };

                // Get new snapshot number
                let snap_num = find_new();

                // Run rebuild
                let run = rebuild(&snapshot, snap_num, &desc);
                match run {
                    Ok(snap_num) => {
                        println!("Tree {} cloned from {}.", snap_num,snapshot);
                    },
                    Err(e) => {
                        eprintln!("{}", e);
                    },
                }
            }
            // Refresh
            Some(("refresh", refresh_matches)) => {
                // Get snapshot value
                let snapshot = if refresh_matches.contains_id("SNAPSHOT") {
                    let snap = refresh_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Run refresh
                refresh(&snapshot).unwrap();
            }
            // Reset
            Some(("reset", _matches)) => {
                // Run reset
                let run = reset();
                match run {
                    Ok(_) => {
                        // Reboot system
                        Command::new("reboot").status().unwrap();
                    },
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Rollback
            Some(("rollback", _matches)) => {
                // Run rollback
                let run = rollback();
                match run {
                    Ok(_) => (),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Chroot run
            Some(("run", run_matches)) => {
                // Get snapshot value
                let snapshot = if run_matches.contains_id("SNAPSHOT") {
                    let snap = run_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Get cmds value
                let cmds: Vec<String> = run_matches.get_many::<String>("COMMAND").unwrap().map(|s| format!("{}", s)).collect();

                // Run chroot
                let run = chroot(&snapshot, cmds);
                match run {
                    Ok(_) => (),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Subvolumes list
            Some(("sub", _matches)) => {
                // Run list_subvolumes
                list_subvolumes();
            }
            // Tree sync
            Some(("sync", sync_matches)) => {
                // Get treename value
                let treename = if sync_matches.contains_id("TREENAME") {
                    let snap = sync_matches.get_one::<i32>("TREENAME").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Optional values
                let live = sync_matches.get_flag("live");

                // Run tree_sync
                let run = tree_sync(&treename, live);
                match run {
                    Ok(_) => println!("Tree {} synced.", treename),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Tree install
            Some(("tinstall", tinstall_matches)) => {
                // Get treename value
                let treename = if tinstall_matches.contains_id("TREENAME") {
                    let snap = tinstall_matches.get_one::<i32>("TREENAME").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Get pkgs value
                let pkgs = if tinstall_matches.contains_id("PACKAGE") {
                    let pkgs: Vec<String> = tinstall_matches.get_many::<String>("PACKAGE").unwrap().map(|s| format!("{}", s)).collect();
                    pkgs
                } else {
                    let pkgs: Vec<String> = Vec::new();
                    pkgs
                };

                // Get profiles value
                let profiles = if tinstall_matches.contains_id("PROFILE") {
                    let profiles: Vec<String> = tinstall_matches.get_many::<String>("PROFILE").unwrap().map(|s| format!("{}", s)).collect();
                    profiles
                } else {
                    let profiles: Vec<String> = Vec::new();
                    profiles
                };

                // Get user_profiles value
                let user_profiles = if tinstall_matches.contains_id("USER_PROFILE") {
                    let user_profiles: Vec<String> = tinstall_matches.get_many::<String>("USER_PROFILE").unwrap().map(|s| format!("{}", s)).collect();
                    user_profiles
                } else {
                    let user_profiles: Vec<String> = Vec::new();
                    user_profiles
                };

                // Optional values
                let noconfirm = tinstall_matches.get_flag("noconfirm");
                let force = tinstall_matches.get_flag("force");
                let secondary = tinstall_matches.get_flag("secondary");

                // Run tree_install
                let run = tree_install(&treename, &pkgs, &profiles, force, &user_profiles, noconfirm, secondary);
                match run {
                    Ok(_) => (),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // tmp (clear tmp)
            Some(("tmp", _matches)) => {
                // Run temp_snapshots_clear
                temp_snapshots_clear().unwrap();
            }
            // Tree
            Some(("tree", _matches)) => {
                //Run tree_show
                tree_show();
            }
            // Tree remove
            Some(("tremove", tremove_matches)) => {
                // Get treename value
                let treename = if tremove_matches.contains_id("TREENAME") {
                    let snap = tremove_matches.get_one::<i32>("TREENAME").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Get pkgs value
                let pkgs = if tremove_matches.contains_id("PACKAGE") {
                    let pkgs: Vec<String> = tremove_matches.get_many::<String>("PACKAGE").unwrap().map(|s| format!("{}", s)).collect();
                    pkgs
                } else {
                    let pkgs: Vec<String> = Vec::new();
                    pkgs
                };

                // Get profiles value
                let profiles = if tremove_matches.contains_id("PROFILE") {
                    let profiles: Vec<String> = tremove_matches.get_many::<String>("PROFILE").unwrap().map(|s| format!("{}", s)).collect();
                    profiles
                } else {
                    let profiles: Vec<String> = Vec::new();
                    profiles
                };

                // Get user_profiles value
                let user_profiles = if tremove_matches.contains_id("USER_PROFILE") {
                    let user_profiles: Vec<String> = tremove_matches.get_many::<String>("USER_PROFILE").unwrap().map(|s| format!("{}", s)).collect();
                    user_profiles
                } else {
                    let user_profiles: Vec<String> = Vec::new();
                    user_profiles
                };

                // Optional value
                let noconfirm = tremove_matches.get_flag("noconfirm");

                // Run tree_remove
                let run = tree_remove(&treename, &pkgs, &profiles, &user_profiles, noconfirm);
                match run {
                    Ok(_) => println!("Tree {} updated.", treename),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Tree run
            Some(("trun", trun_matches)) => {
                // Get snapshot value
                let treename = if trun_matches.contains_id("TREENAME") {
                    let snap = trun_matches.get_one::<i32>("TREENAME").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let treename = get_current_snapshot();
                    treename
                };

                // Get cmds value
                let cmds: Vec<String> = trun_matches.get_many::<String>("COMMAND").unwrap().map(|s| format!("{}", s)).collect();

                // Run tree_run
                for cmd in cmds {
                    let run = tree_run(&treename, &cmd);
                    match run {
                        Ok(_) => println!("Tree {} updated.", treename),
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            // Tree upgrade
            Some(("tupgrade", tupgrade_matches)) => {
                // Get treename value
                let treename = if tupgrade_matches.contains_id("TREENAME") {
                    let snap = tupgrade_matches.get_one::<i32>("TREENAME").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Run tree_upgrade
                let run = tree_upgrade(&treename);
                match run {
                    Ok(_) => println!("Tree {} updated.", treename),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Uninstall package(s) from a snapshot
            Some(("uninstall", uninstall_matches)) => {
                // Get snapshot value
                let snapshot = if uninstall_matches.contains_id("SNAPSHOT") {
                    let snap = uninstall_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Get pkgs value
                let pkgs = if uninstall_matches.contains_id("PACKAGE") {
                    let pkgs: Vec<String> = uninstall_matches.get_many::<String>("PACKAGE").unwrap().map(|s| format!("{}", s)).collect();
                    pkgs
                } else {
                    let pkgs: Vec<String> = Vec::new();
                    pkgs
                };

                // Get profile value
                let profile = if uninstall_matches.contains_id("PROFILE") {
                    let profile = uninstall_matches.get_many::<String>("PROFILE").unwrap().map(|s| format!("{}", s)).collect();
                    profile
                } else {
                    let profile = String::new();
                    profile
                };

                // Get user_profile value
                let user_profile = if uninstall_matches.contains_id("USER_PROFILE") {
                    let user_profile = uninstall_matches.get_many::<String>("USER_PROFILE").unwrap().map(|s| format!("{}", s)).collect();
                    user_profile
                } else {
                    let user_profile = String::new();
                    user_profile
                };

                // Optional values
                let live = uninstall_matches.get_flag("live");
                let noconfirm = uninstall_matches.get_flag("noconfirm");

                // Run uninstall_triage
                uninstall_triage(&snapshot, live, pkgs, &profile, &user_profile, noconfirm).unwrap();
            }
            // Unlock a snapshot
            Some(("unlock", unlock_matches)) => {
                // Get snapshot value
                let snapshot = if unlock_matches.contains_id("SNAPSHOT") {
                    let snap = unlock_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Run snapshot_unlock
                let run = snapshot_unlock(&snapshot);
                match run {
                    Ok(_) => (),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Upgrade a snapshot
            Some(("upgrade", upgrade_matches)) => {
                // Get snapshot value
                let snapshot = if upgrade_matches.contains_id("SNAPSHOT") {
                    let snap = upgrade_matches.get_one::<i32>("SNAPSHOT").unwrap();
                    let snap_to_string = format!("{}", snap);
                    snap_to_string
                } else {
                    let snap = get_current_snapshot();
                    snap
                };

                // Optional value
                let noconfirm = upgrade_matches.get_flag("noconfirm");

                // Run upgrade
                let run = upgrade(&snapshot, false, noconfirm);
                match run {
                    Ok(_) => {},
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Ash version
            Some(("version", _matches)) => {
                // Run ash_version
                let run = ash_version();
                match run {
                    Ok(_) => (),
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Which snapshot(s) contain a package
            Some(("whichsnap", whichsnap_matches)) => {
                // Get pkgs value
                let pkgs: Vec<String> = whichsnap_matches.get_many::<String>("PACKAGE").unwrap().map(|s| format!("{}", s)).collect();

                // Run which_snapshot_has
                which_snapshot_has(pkgs);
            }
            // Which deployment is active
            Some(("whichtmp", _matches)) => {
                // Run print_tmp
                println!("{}", print_tmp());
            }
           _=> unreachable!(), // If all subcommands called, anything else is unreachable
        }
    }
}
