/**
 * Copyright 2022 Thomas Dons
 *
 * Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
 * following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
 * disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
 * following disclaimer in the documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
 * products derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
 * INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
 * SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
#include <errno.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include <sys/wait.h>

// #include <bsd/string.h>

#include "jsmn.h"

/* Download files to "/tmp/git-lfs-agent-scp-$OID" before handing them over to git-lfs */
static const char tmpfile_prefix[] = "/tmp/git-lfs-agent-scp-";
/* An input line of text read from stdin */
static char line[1024];
/* Json tokenization of line */
static jsmntok_t tokens[128];

#define OID_LENGTH 64
#define LAST_JSON_TOKEN (-1)

// if windows or linux, define strlcpy and strlcat
#if defined(_WIN32) || defined(_WIN64) || defined(__linux__)

size_t strlcpy(char *dst, const char *src, size_t n) {
    char *p = dst;

    if (n != 0) {
        for (; --n != 0; p++, src++) {
            if ((*p = *src) == '\0')
                return p - dst;
        }
        *p = '\0';
    }
    return (p - dst) + strlen(src);
}

size_t strlcat(char *dst, const char *src, size_t n) {
    char *p = dst;

    while (n != 0 && *p != '\0') {
        p++;
        n--;
    }
    if (n != 0) {
        for (; --n != 0; p++, src++) {
            if ((*p = *src) == '\0')
                return p - dst;
        }
        *p = '\0';
    }
    return (p - dst) + strlen(src);
}

#endif

/**
 * Write a message to stderr.
 *
 * These messages can be inspected when running git with GIT_TRACE enabled:
 * $ GIT_TRACE=1 git push
 */
static void debuglog(const char *format, ...)
{
	va_list args;
	va_start(args, format);
	vfprintf(stderr, format, args);
	va_end(args);
}

static void panic(const char *error)
{
	fprintf(stderr, "panic: %s\n", error);
	exit(1);
}

/* Documentation: https://github.com/git-lfs/git-lfs/blob/main/docs/custom-transfers.md */
typedef enum {
	UNKNOWN,
	INIT,
	UPLOAD,
	DOWNLOAD,
	TERMINATE,
} git_lfs_agent_event_t;

/**
 * Look up a json string in a list of json tokens (largely disregarding the json structure)
 *
 * @param key the string key we search for
 */
static char *find_string(const char *key)
{
	for (const jsmntok_t *t = tokens; (int)t->type != LAST_JSON_TOKEN; t++) {
		/*
		 * Look for an object key with name equal to key.
		 * The "size == 1" condition ensures we only look for json keys.
		 */
		if (!(t->type == JSMN_STRING && strcmp(line + t->start, key) == 0 && t->size == 1)) {
			continue;
		}
		/* Expect the value to be a string. */
		if ((t + 1)->type != JSMN_STRING) {
			panic("find_string found the key, but the associated value is not a string");
		}
		return line + (t + 1)->start;
	}
	return 0;
}

/**
 * Determine the event type given a message sent by git-lfs.
 */
static git_lfs_agent_event_t get_event_type()
{
	const char *event = find_string("event");
	if (event == 0) {
		panic("expected to find the \"event\" key");
	}
	switch (*event) {
	/* Hack: we're only inspecting the first character.  Oh well. */
	case 'i':
		return INIT;
	case 'd':
		return DOWNLOAD;
	case 'u':
		return UPLOAD;
	case 't':
		return TERMINATE;
	}
	return UNKNOWN;
}

/**
 * Zero-terminates individual json string values in a buffer.
 *
 * The jsmn json parser returns a list of json tokens (jsmntok_t), these tokens do not
 * contain copies of parsed items but instead contain pointers into the original string
 * fed to the parser (line in our case).  A jsmntok_t of type string has a start and
 * an end index.  We set the character at the end index (which is always a ") to zero
 * to make the strings in the json C strings.  This makes working with the string in
 * the json easier in the rest of the program.
 */
static void zero_terminate_strings()
{
	for (const jsmntok_t *t = tokens; (int)t->type != LAST_JSON_TOKEN; t++) {
		if (t->type != JSMN_STRING) {
			continue;
		}
		if (line[t->end] != '"') {
			panic("zero_terminate_strings: expected a \"");
		}
		line[t->end] = '\0';
	}
}

/**
 * Executes another program and waits for it to finish executing.
 *
 * @return 0 on success
 */
static int exec(char **argv)
{
	if (!fork()) {
		if (execvp(argv[0], argv) == -1) {
			fprintf(stderr, "execvp failed, errno=%d errstr=%s\n", errno, strerror(errno));
		}
		return 1;
	} else {
		int child_exit_code;
		if (wait(&child_exit_code) == -1) {
			fprintf(stderr, "wait failed, errno=%d errstr=%s\n", errno, strerror(errno));
			return 1;
		}
		if (WIFEXITED(child_exit_code) && WEXITSTATUS(child_exit_code) == 0) {
			/* Normal execution */
			return 0;
		}
		if (WIFEXITED(child_exit_code)) {
			fprintf(stderr, "child exited abnormally with code=%d\n", WEXITSTATUS(child_exit_code));
			return 1;
		}
		if (WIFSIGNALED(child_exit_code)) {
			fprintf(stderr, "child received signal=%s\n", strsignal(WTERMSIG(child_exit_code)));
			return 1;
		}
		fprintf(stderr, "child exited abnormally\n");
		return 1;
	}
}

/**
 * git-lfs-agent-scp
 *
 * argv must contain exactly 1 argument that points to the location where objects tracked by git-lfs
 * will be stored/retrieved by scp.
 */
int main(int argc, char *argv[])
{
	char local_tmp_file[sizeof(tmpfile_prefix) + OID_LENGTH + 1] = "\0";
	char remote_path[1024] = "\0";
	jsmn_parser parser;
	int num_tokens;
	int done = 0;
	size_t remote_path_prefix_len;
	char *scp_argv[] = {"scp", "-B", 0, 0, 0};
	char **scp_src = scp_argv + 2;
	char **scp_dst = scp_argv + 3;

	strcpy(local_tmp_file, tmpfile_prefix);

	if (argc != 2) {
		panic("expecting the scp destination as argument");
	}
	remote_path_prefix_len = strlcpy(remote_path, argv[1], sizeof(remote_path) - 1);
	if (remote_path_prefix_len >= sizeof(remote_path) - 1) {
		panic("destination path is too long");
	}
	if (remote_path_prefix_len == 0) {
		panic("destination path is empty");
	}
	if (remote_path[remote_path_prefix_len - 1] != '/') {
		remote_path[remote_path_prefix_len] = '/';
		remote_path[remote_path_prefix_len + 1] = '\0';
		remote_path_prefix_len += 1;
	}
	/* Ensure buffer has size for an oid */ 
	if (remote_path_prefix_len + OID_LENGTH + 1 >= sizeof(remote_path)) {
		panic("destination path is too long");
	}

	/*
	 * git-lfs sends us a separate json object on every line (https://jsonlines.org/).
	 * we respond in kind.  enable line buffered io on stdout to make sure the process
	 * doesn't stall because git-lfs is waiting for a reply that is stuck in an io
	 * buffer.
	 */
	setvbuf(stdout, NULL, _IOLBF, BUFSIZ);

	while (fgets(line, sizeof(line), stdin) != 0) {
		debuglog("read input line=%s", line);

		jsmn_init(&parser);
		num_tokens = jsmn_parse(&parser, line, strnlen(line, sizeof(line)), tokens, sizeof(tokens) - 1);
		if (num_tokens < 0) {
			panic("could not parse json");
		}
		tokens[num_tokens].type = LAST_JSON_TOKEN;
		zero_terminate_strings();

		switch (get_event_type()) {
		case INIT:
			printf("{}\n");
			continue;
		case DOWNLOAD: {
			char *oid = find_string("oid");

			if (oid == 0) {
				panic("download message missing oid");
			}
			if (strnlen(oid, OID_LENGTH + 1) >= OID_LENGTH + 1) {
				panic("oid longer than expected");
			}
			strcpy(remote_path + remote_path_prefix_len, oid);
			strcpy(local_tmp_file + sizeof(tmpfile_prefix) - 1, oid);

			debuglog("download oid=%s from=%s to=%s\n", oid, remote_path, local_tmp_file);

			*scp_src = remote_path;
			*scp_dst = local_tmp_file;
			if (exec(scp_argv) != 0) {
				panic("scp failed for download");
			}

			printf("{\"event\":\"complete\",\"oid\":\"%s\",\"path\":\"%s\"}\n", oid, local_tmp_file);
			continue;
		}
		case UPLOAD: {
			char *oid = find_string("oid");
			char *local_file = find_string("path");

			if (oid == 0 || local_file == 0) {
				panic("upload message missing oid or path");
			}
			if (strnlen(oid, OID_LENGTH + 1) >= OID_LENGTH + 1) {
				panic("oid longer than expected");
			}
			strcpy(remote_path + remote_path_prefix_len, oid);

			debuglog("upload oid=%s from=%s to=%s\n", oid, local_file, remote_path);

			*scp_src = local_file;
			*scp_dst = remote_path;
			if (exec(scp_argv) != 0) {
				panic("scp failed for upload");
			}

			printf("{\"event\":\"complete\",\"oid\":\"%s\"}\n", oid);
			continue;
		}
		case TERMINATE:
			done = 1;
			break;
		case UNKNOWN:
			panic("encountered unknown \"event\" type");
		}
	}

	if (!done) {
		panic("reached EOF before receiving terminate event.");
	}

	return 0;
}
