Skylane
=======

`skylane` is implementation of Wayland protocol written in Rust.

Project consists of three repositories:

 - [`skylane`](https://github.com/perceptia/skylane) - core protocol implementation

 - [`skylane_scanner`](https://github.com/perceptia/skylane_scanner) - generates marshalling code
   from XML protocol description (equivalent to `wayland-scanner`)

 - [`skylane_protocols`](https://github.com/perceptia/skylane_protocols) - protocol marshalling code
   generated using `skylane_scanner` + some glue code

Documentation
-------------

Documentation can be found on [docs.rs](https://docs.rs/skylane).

Project
-------

`skylane` is developed as part of [`perceptia`](https://github.com/perceptia/perceptia) project.
Most up to date usage examples can be found there.

License
-------

`skylane` is released under the [MIT License](https://opensource.org/licenses/MIT).
