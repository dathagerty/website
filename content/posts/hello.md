---
title: A True Hello World
publishDate: 2024-10-14
lastEdit: 2024-10-14
tags: [golang, meta, blogging, code, practices]
summary: A tour of this blog and how it works
---

Welcome to this blog! I'm on my, like, fifth? rewrite of this thing, and I'd like to take some time to talk about what that means. First off, this is functionally the same as my old blog. It's still Go, still a single binary. The biggest difference is that I *massively* reduced my dependency footprint, I upgraded to the latest version of Go, and I added support for Markdown. Let's break that down.

### Reducing Dependencies

This goes hand-in-hand with upgrading to a newer version of Go. I now have access to goodies like [log/slog](https://pkg.go.dev/log/slog) and the new [net/http](https://pkg.go.dev/net/http#ServeMux) handling. The other big thing is I moved to using the built-in [flag](https://pkg.go.dev/flag) module over [spf13/pflag](https://pkg.go.dev/github.com/spf13/pflag). I found that this massively simplified my code. I no longer feel the need to pull in a router like [chi](https://pkg.go.dev/github.com/go-chi/chi/v5) with the new `http.ServeMux` implementation. I get just about all I need to serve up some HTML. For example, I can do this with the new routing:

```golang
http.Handle("GET /", NotFoundHandler)
http.Handle("GET /{$}", RootHandler)
http.Handle("GET /about", AboutHandler)
```

What this does is pretty neat. It sets up three routes, and how they match is the secret sauce. First, they all explicitly set that they respond to `GET` requests only, no more needing to check request methods in handlers. Then, there is the middle route, that connects `/{$}` to `RootHandler`. The `{$}` bit signifies the end of the route for pattern matching purposes, so this route will match exactly `/` and nothing else. This is needed because the first route I declared, the `GET /` route, is actually a catch-all for any route I haven't defined. So if I get a request for some path that my blog doesn't know about, it will not match anything except that `NotFoundHandler` route. The only downside here is I will serve full HTML to the bots trying to exploit my server by requesting weird paths.

### Upgrading Go

Yeah, it's a good upgrade. If you're not on at least 1.22, I'd recommend starting to use that if possible, especially for new projects. As it is, I'm only using external dependencies to parse Markdown. Everything else I'm doing is with the standard library.

### Supporting Markdown

Honestly, this is only because I don't want to hand-write a bunch of HTML. Originally that was my plan for this blog, but it got tedious and it reduced my motivation to write. It's not like I even do anything super fancy with HTML (for now), I could probably just cram all the text I want in an HTML file and call it a day. But something about Markdown just makes writing seem easier for this blog. And it's easier to add in front-matter, though I'm not a huge fan of using YAML for anything. But supporting Markdown is just the first step on the road to writing my own markup syntax and parser (a man needs hobbies).
