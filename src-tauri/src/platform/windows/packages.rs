//! Store (MSIX) apps can't be started from their .exe inside C:\Program Files\WindowsApps
//! (Windows denies it), and that folder name changes with every update. Map such a path to
//! the app's AppUserModelID through Windows' package manager, which survives updates.
use windows::core::HSTRING;
use super::super::InstalledApp;
use windows::Management::Deployment::PackageManager;

/// "…\WindowsApps\5319275A.WhatsAppDesktop_2.2640.100.0_x64__cv1g1gvanyjgm\WhatsApp.Root.exe"
/// -> ("5319275A.WhatsAppDesktop_cv1g1gvanyjgm", "WhatsApp.Root.exe"). Pure, so it's tested.
pub fn family_of(path: &str) -> Option<(String, String)> {
    let low = path.to_lowercase();
    let at = low.find("\\windowsapps\\")? + "\\windowsapps\\".len();
    let rest = &path[at..];
    let (folder, inner) = rest.split_once('\\')?;
    let (left, publisher) = folder.rsplit_once("__")?;
    let name = left.split('_').next()?;
    (!name.is_empty() && !publisher.is_empty()).then(|| (format!("{name}_{publisher}"), inner.to_string()))
}

pub fn aumid_for_path(path: &str) -> Option<String> {
    let (family, inner) = family_of(path)?;
    super::com_init();
    let pm = PackageManager::new().ok()?;
    let pkgs = pm.FindPackagesByUserSecurityIdPackageFamilyName(&HSTRING::new(), &HSTRING::from(&family)).ok()?;
    for pkg in pkgs {
        let Ok(entries) = pkg.GetAppListEntriesAsync().and_then(|o| o.join()) else { continue };
        let ids: Vec<String> = entries.into_iter().filter_map(|e| e.AppUserModelId().ok().map(|s| s.to_string())).collect();
        // Several apps in one package: prefer the entry whose manifest Executable is our .exe.
        let manifest = pkg.InstalledPath().ok()
            .and_then(|p| std::fs::read_to_string(format!("{p}\\AppxManifest.xml")).ok()).unwrap_or_default();
        if let Some(id) = ids.iter().find(|id| executable(&manifest, app_id(id)).is_some_and(|x| x.eq_ignore_ascii_case(&inner))) {
            return Some(id.clone());
        }
        if let Some(first) = ids.into_iter().next() { return Some(first); }
    }
    None
}

fn app_id(aumid: &str) -> &str { aumid.rsplit('!').next().unwrap_or("") }

/// The `Executable` of the `<Application Id="…">` in an AppxManifest.xml. Attributes come in
/// any order and often on their own lines.
pub fn executable(manifest: &str, id: &str) -> Option<String> {
    let attr = |el: &str, name: &str| {
        let key = format!("{name}=\"");
        el.match_indices(&key).find(|(i, _)| el[..*i].ends_with(char::is_whitespace))
            .and_then(|(i, _)| el[i + key.len()..].split('"').next()).map(str::to_string)
    };
    manifest.split("<Application").skip(1)
        .map(|rest| &rest[..rest.find('>').unwrap_or(rest.len())])
        .filter(|el| el.starts_with(char::is_whitespace))
        .find(|el| attr(el, "Id").as_deref() == Some(id))
        .and_then(|el| attr(el, "Executable"))
}

/// Store apps that show in the Start menu, for the app pickers and search.
pub fn installed() -> Vec<InstalledApp> {
    super::com_init();
    let Ok(pkgs) = PackageManager::new().and_then(|pm| pm.FindPackagesByUserSecurityId(&HSTRING::new())) else { return vec![] };
    let mut out = Vec::new();
    for pkg in pkgs {
        if pkg.IsFramework().unwrap_or(true) || pkg.IsResourcePackage().unwrap_or(true) { continue; }
        let Ok(entries) = pkg.GetAppListEntriesAsync().and_then(|o| o.join()) else { continue };
        let Ok(dir) = pkg.InstalledPath().map(|p| p.to_string()) else { continue };
        let manifest = std::fs::read_to_string(format!("{dir}\\AppxManifest.xml")).unwrap_or_default();
        for e in entries {
            let Ok(aumid) = e.AppUserModelId().map(|s| s.to_string()) else { continue };
            let Ok(name) = e.DisplayInfo().and_then(|d| d.DisplayName()).map(|s| s.to_string()) else { continue };
            if name.is_empty() { continue; }
            // No Executable (a hosted or web app): the package folder stands in as its path.
            let path = executable(&manifest, app_id(&aumid)).map_or_else(|| dir.clone(), |x| format!("{dir}\\{x}"));
            out.push(InstalledApp { name, path, args: String::new(), aumid: Some(aumid) });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn executable_reads_the_matching_application() {
        let m = r#"<Package><Identity Name="SpotifyAB.SpotifyMusic" Version="1.0.0.0"/>
          <Applications>
            <Application Id="Helper" Executable="helper.exe"/>
            <Application
              Executable="Spotify.exe"
              EntryPoint="Windows.FullTrustApplication"
              Id="Spotify">
            </Application>
            <Application uap10:HostId="Spotify" Id="Web"/>
          </Applications></Package>"#;
        assert_eq!(super::executable(m, "Spotify").as_deref(), Some("Spotify.exe"));
        assert_eq!(super::executable(m, "Helper").as_deref(), Some("helper.exe"));
        assert_eq!(super::executable(m, "Web"), None); // no Executable; HostId isn't Id
        assert_eq!(super::executable(m, "Missing"), None);
    }

    /// The Store apps on this PC and what listing them costs. By hand:
    /// cargo test store_apps_here -- --ignored --nocapture
    #[test]
    #[ignore]
    fn store_apps_here() {
        let t = std::time::Instant::now();
        let a = super::installed();
        println!("{} Store apps in {:?}", a.len(), t.elapsed());
        for x in &a { println!("  {} | {} | {:?}", x.name, x.path, x.aumid); }
    }

    #[test]
    fn family_from_versioned_path() {
        assert_eq!(super::family_of(r"C:\Program Files\WindowsApps\5319275A.WhatsAppDesktop_2.2640.100.0_x64__cv1g1gvanyjgm\WhatsApp.Root.exe"),
            Some(("5319275A.WhatsAppDesktop_cv1g1gvanyjgm".into(), "WhatsApp.Root.exe".into())));
        assert_eq!(super::family_of(r"C:\Program Files\WindowsApps\Claude_3.1623.3.0_x64__pzs8sxrjxfjjc\app\claude.exe"),
            Some(("Claude_pzs8sxrjxfjjc".into(), r"app\claude.exe".into())));
        assert_eq!(super::family_of(r"C:\Program Files\Google\Chrome\Application\chrome.exe"), None);
    }
}
