#include <windows.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    HKEY key;
    if (RegOpenKeyExW(HKEY_CURRENT_USER, L"Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings", 0, KEY_QUERY_VALUE, &key) != ERROR_SUCCESS) {
        return 0;
    }
    DWORD enabled = 0, size = sizeof(enabled), type = 0;
    if (RegQueryValueExW(key, L"ProxyEnable", NULL, &type, (LPBYTE)&enabled, &size) != ERROR_SUCCESS || type != REG_DWORD || enabled == 0) {
        RegCloseKey(key);
        return 0;
    }
    WCHAR server[1024];
    size = sizeof(server);
    if (RegQueryValueExW(key, L"ProxyServer", NULL, &type, (LPBYTE)server, &size) != ERROR_SUCCESS || type != REG_SZ || size == 0) {
        RegCloseKey(key);
        return 0;
    }
    RegCloseKey(key);
    server[(size / 2) - 1] = L'\0';
    if (server[0] == L'\0') {
        return 0;
    }
    if (wcsstr(server, L"://") == NULL) {
        fwprintf(stdout, L"pub const SYSTEM_PROXY_URL: Option<&str> = Some(\"http://%s\");\n", server);
    } else {
        fwprintf(stdout, L"pub const SYSTEM_PROXY_URL: Option<&str> = Some(\"%s\");\n", server);
    }
    return 0;
}
