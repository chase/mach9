/* Tell the host which macOS process is running this Windows child. Keep it
 * alive until the host has checked its bundle. */
#define WIN32_LEAN_AND_MEAN
#include <stddef.h>
#include <stdio.h>
#include <windows.h>
#include <wine/server.h>

int main(void)
{
    __typeof__(&wine_server_call) call;
    int pid = -1;
    char ack;
    DWORD count;

    call = (__typeof__(call))(void *)GetProcAddress(GetModuleHandleW(L"ntdll.dll"), "wine_server_call");
    if (!call) return 1;
    SERVER_START_REQ(get_thread_times)
    {
        req->handle = wine_server_obj_handle(GetCurrentThread());
        if (!call(&__req)) pid = reply->unix_pid;
    }
    SERVER_END_REQ;
    if (pid <= 0) return 1;
    printf("wine-build smoke test: child pid %d\n", pid);
    fflush(stdout);
    return !(ReadFile(GetStdHandle(STD_INPUT_HANDLE), &ack, 1, &count, NULL) && count == 1 && ack == '+');
}
