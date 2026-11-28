/* Check the loader at the given native PID, including the signature of its
 * enclosing app. Wine can rewrite argv, so ask the kernel for the image. */
#include <libproc.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

int main(int argc, char **argv)
{
    static const char loader[] = "/Contents/MacOS/wine";
    char path[PROC_PIDPATHINFO_MAXSIZE], *name;
    size_t len;

    if (argc != 3) return 1;
    if (proc_pidpath(atoi(argv[1]), path, sizeof(path)) <= 0)
    {
        perror("proc_pidpath");
        return 1;
    }
    len = strlen(path);
    if (len >= sizeof(loader) && !strcmp(path + len - strlen(loader), loader))
    {
        path[len - strlen(loader)] = 0;
        name = strrchr(path, '/');
        if (name && !strcmp(name + 1, argv[2]))
        {
            execl("/usr/bin/codesign", "codesign", "--verify", "--strict", path, (char *)NULL);
            perror("codesign");
            return 1;
        }
    }
    fprintf(stderr, "Expected %s at PID %s, got %s\n", argv[2], argv[1], path);
    return 1;
}
