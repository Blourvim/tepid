````markdown
# Tepid

Tepid is a TCP proxy that blocks requests using blacklisted API keys. It sits in
front of an existing HTTP service, checks the `Authorization` header against a
revocation list, and forwards everything else untouched.

Still under development.

## Description

At startup Tepid hydrates key hashes from a Postgres table into an in-memory bloom
filter. Each incoming connection is checked against the filter.

Whatever string is in the `Authorization` header is treated as the key hash.
A `Bearer ` prefix is stripped if present.

## ROADMAP

This is not production grade, do not use in production. There is still a lot to
do, if you need a feature let me know.

- [ ] Api controls and dynamic blacklist updates.
- [ ] Allow outside control via an api.
- [ ] Implement better algorithms.
- [ ] Update with a real http parser.
- [ ] Create benchmarks
- [ ] Better deployment.
- [ ] XDP/ eBPF implementation.

## Authors

İbrahim Mert Yıldız — blourvim@gmail.com

## License

This project is licensed under the GNU Affero General Public License v3.0 - see
the [LICENSE](LICENSE) file for details.
````
