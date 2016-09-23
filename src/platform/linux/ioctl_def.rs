// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

// Some ioctls are exported by ioctl crate only for x86_64, so we have to define them anyway.
// Diffing linux/input.h across different architectures (i686, x86_64 and arm) didn't show any
// difference, so it looks like conditional compilation is not needed.

use ioctl::input_id;

ioctl!(read eviocgid with b'E', 0x02; /*struct*/ input_id);
ioctl!(read eviocgeffects with b'E', 0x84; ::libc::c_int);
ioctl!(write eviocrmff with b'E', 0x81; ::libc::c_int);
