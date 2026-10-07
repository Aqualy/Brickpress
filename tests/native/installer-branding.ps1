param(
    [string]$Application = 'src-tauri/target/release/brickpress.exe',
    [string]$Installer = 'src-tauri/target/release/bundle/nsis/Brickpress_1.0.0_x64-setup.exe',
    [string]$Icon = 'src-tauri/icons/icon.ico'
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
# Read PE resources directly, avoiding Explorer's file-icon cache. Loading as a
# data file does not start the application or installer, or install anything.
Add-Type @'
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.InteropServices;
public static class BrickpressIconResources {
    private delegate bool ResourceNameCallback(IntPtr module, IntPtr type, IntPtr name, IntPtr parameter);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr LoadLibraryEx(string path, IntPtr file, uint flags);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool EnumResourceNames(IntPtr module, IntPtr type, ResourceNameCallback callback, IntPtr parameter);
    [DllImport("kernel32.dll")]
    private static extern bool FreeLibrary(IntPtr module);
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr LoadImage(IntPtr module, IntPtr name, uint type, int width, int height, uint flags);
    [DllImport("user32.dll")]
    public static extern bool DestroyIcon(IntPtr icon);
    public static IntPtr[] Read(string path) {
        var module = LoadLibraryEx(path, IntPtr.Zero, 0x22);
        if (module == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
        var icons = new List<IntPtr>();
        int error = 0;
        try {
            bool result = EnumResourceNames(module, new IntPtr(14), (handle, type, name, parameter) => {
                var icon = LoadImage(handle, name, 1, 32, 32, 0);
                if (icon == IntPtr.Zero) { error = Marshal.GetLastWin32Error(); return false; }
                icons.Add(icon);
                return true;
            }, IntPtr.Zero);
            if (!result || icons.Count == 0) {
                foreach (var icon in icons) DestroyIcon(icon);
                throw new Win32Exception(error != 0 ? error : Marshal.GetLastWin32Error());
            }
            return icons.ToArray();
        } finally { FreeLibrary(module); }
    }
}
'@
$expectedIcon = [System.Drawing.Icon]::new((Resolve-Path $Icon).Path, 32, 32)
$expected = $expectedIcon.ToBitmap()
try {
    foreach ($file in @($Application, $Installer)) {
        $handles = [BrickpressIconResources]::Read((Resolve-Path $file).Path)
        try {
            foreach ($handle in $handles) {
                $actualIcon = [System.Drawing.Icon]::FromHandle($handle)
                $actual = $actualIcon.ToBitmap()
                try {
                    $different = 0
                    for ($y = 0; $y -lt 32; $y++) {
                        for ($x = 0; $x -lt 32; $x++) {
                            $a = $actual.GetPixel($x, $y)
                            $b = $expected.GetPixel($x, $y)
                            if ($a.A -ne $b.A -or ($b.A -gt 0 -and $a.ToArgb() -ne $b.ToArgb())) { $different++ }
                        }
                    }
                    if ($different) { throw "An embedded icon in $file differs from the supplied custom icon ($different pixels)." }
                } finally { $actual.Dispose(); $actualIcon.Dispose() }
            }
            Write-Output "Verified all $($handles.Length) embedded icon group(s) in $file against the custom artwork."
        } finally { foreach ($handle in $handles) { [BrickpressIconResources]::DestroyIcon($handle) | Out-Null } }
    }
} finally { $expected.Dispose(); $expectedIcon.Dispose() }
