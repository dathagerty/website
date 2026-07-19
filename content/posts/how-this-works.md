---
title: How This Site Works
publishDate: 2024-11-22
lastEdit: 2024-11-21
draft: true
tags: [golang, meta, blogging, code, practices]
summary: A deep dive into the pipeline that builds this site
---

Picking up from my [previous post](https://dathagerty.com/blag/hello), I'd like to talk in more detail about how this site works.
Mostly because this is pretty cool (no bias, promise), and also because I think it will serve as a good example for others.
The big things I want to hit are:

- How the site starts up
- How the site is built
- How the site is deployed

### Start Me Up

This is a simple program, so I don't use a crazy project layout.
I just start the program in a `main.go` file in the root of the project.
It looks like this:

```go
package main

import (
	"context"
	"io"
	"os"
	"os/signal"
	"syscall"

	"git.sr.ht/~gloatingfiddle/dathagerty/internal/server"
)

func main() {
	base := context.Background()
	os.Exit(run(base, os.Args[1:], os.Stdout))
}

func run(ctx context.Context, args []string, stdout io.Writer) int {
	ctx, cancel := signal.NotifyContext(ctx, syscall.SIGINT, syscall.SIGKILL, syscall.SIGTERM)
	defer cancel()

	return server.Run(ctx, args, stdout)
}
```

You may recognize this if you're read Mat Ryer's [post](https://grafana.com/blog/2024/02/09/how-i-write-http-services-in-go-after-13-years/) about how he writes HTTP services in Go.
I've really come to love the idea of having something like a `main` function that can return an error/exit code.
I specify signals explicitly, but there is no real reason for it other than I want it to be clear what signals can be used to stop the process.

Starting up the actual server is pretty simple as well.
In the `server` package, I parse command line flags and do all of the actual set up work for the program.
That is fairly standard, so I'm not going to get into it here.
If you are interested in the code, it's in [server.go](https://git.sr.ht/~gloatingfiddle/dathagerty/tree/master/item/internal/server/server.go).
The only interesting bit, I feel, in that code is how I set up the logger and at what level it emits logs.

```go
flags.Func("level", "log level to use [info|debug (default: info)]", func(arg string) error {
	switch arg {
	case "debug":
		s.logLevel.Set(slog.LevelDebug)
	case "info":
		s.logLevel.Set(slog.LevelInfo)
	default:
		return fmt.Errorf("unknown log level: %s", arg)
	}
	return nil
})
```

This is probably overkill for managing the log level, but it is expandable for future features.
I could just construct a log handler and have the level be static and only able to be set at startup.
But with this setup, I could add a little bit of code to potentially change the log level when a certain signal is received.
I know YAGNI is a thing, but there's a difference between laying a foundation for future work and building out entire sets of features before you know you will need them.
To put in another way, yeah I'm not using the dynamic log level feature now, but it is also a reasonable way to solve the problem even if I never write code to dynamically change the program's log level.
