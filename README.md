<!-- Improved compatibility of back to top link: See: https://github.com/othneildrew/Best-README-Template/pull/73 -->
<a name="readme-top"></a>
<!--
*** Thanks for checking out the Best-README-Template. If you have a suggestion
*** that would make this better, please fork the repo and create a pull request
*** or simply open an issue with the tag "enhancement".
*** Don't forget to give the project a star!
*** Thanks again! Now go create something AMAZING! :D
-->



<!-- PROJECT SHIELDS -->
<!--
*** I'm using markdown "reference style" links for readability.
*** Reference links are enclosed in brackets [ ] instead of parentheses ( ).
*** See the bottom of this document for the declaration of the reference variables
*** for contributors-url, forks-url, etc. This is an optional, concise syntax you may use.
*** https://www.markdownguide.org/basic-syntax/#reference-style-links
-->
[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]



<!-- PROJECT LOGO -->
<br />
<div align="center">

  <h3 align="center">Guess The Song - SERVER</h3>

  <p align="center">
    The server behind Guess The Song, it downloads songs, manages lobbys and handles game-flow
    <br />
    <br />
    <a href="https://gts.bltz.cloud">View Demo</a>
    ·
    <a href="https://github.com/VirusBLITZ/guess_the_song_backend/issues">Report Bug</a>
    ·
    <a href="https://github.com/VirusBLITZ/guess_the_song_backend/issues">Request Feature</a>
  </p>
</div>



<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#built-with">Built With</a></li>
      </ul>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#installation">Installation</a></li>
      </ul>
    </li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#contributing">Contributing</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#contact">Contact</a></li>
    <li><a href="#acknowledgments">Acknowledgments</a></li>
  </ol>
</details>



<!-- ABOUT THE PROJECT -->
## About The Project

![Guess The Song protocol example](https://github.com/VirusBLITZ/guess_the_song_backend/assets/58221423/b3565404-7d65-4db2-949c-8531574c54d6)

The server communicates over a simple websocket protocol that I came up with myself, it allows for a variety of clients and platforms!

Some possible clients that could easily be made:
* A phone app that can be used in group settings
* A desktop app for optimal performance and usability
* A terminal client for advanced users :smile:

<p align="right">(<a href="#readme-top">back to top</a>)</p>



### Built With

This section should list any major frameworks/libraries used to bootstrap your project. Leave any add-ons/plugins for the acknowledgements section. Here are a few examples.

* [![Rust][rust-lang.org]][Rust-url]

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- GETTING STARTED -->
## Getting Started

To set this up locally, you will need the following:

### Prerequisites


* rust [installation](https://www.rust-lang.org/tools/install)
* yt-dlp
  ```sh
  pip install yt-dlp
  # ensure the program is in your PATH and can be executed
  ```

### Installation

_Below is an example of how you can instruct your audience on installing and setting up your app. This template doesn't rely on any external dependencies or services._

1. Clone the repo
   ```sh
   git clone https://github.com/your_username_/Project-Name.git
   ```
2. Install cargo packages
   ```sh
   cargo c
   ```

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- USAGE EXAMPLES -->
## Usage

Running the server ⚙️

```sh
cargo run
```

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- ROADMAP -->
## Roadmap

- [x] Lobby system
- [x] Song downloads
- [x] Song guessing flow

- [ ] Add more game modes
- [ ] Allow changing the max guessing time

See the [open issues](https://github.com/VirusBLITZ/guess_the_song_backend/issues) for a full list of proposed features (and known issues).

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- CONTRIBUTING -->
## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make this better, please fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement".
Don't forget to give the project a star! Thanks again!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- LICENSE -->
## License

Distributed under the MIT License. See `LICENSE.txt` for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- CONTACT -->
## Contact

Valentin - [@v_bltz](https://twitter.com/v_bltz)

Project Link: [https://github.com/VirusBLITZ/guess_the_song_backend](https://github.com/VirusBLITZ/guess_the_song_backend)

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- ACKNOWLEDGMENTS -->
## Acknowledgments

* @FoggySky - ideas and opinions
* [Img Shields](https://shields.io)

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[contributors-shield]: https://img.shields.io/github/contributors/VirusBLITZ/guess_the_song_backend.svg?style=for-the-badge
[contributors-url]: https://github.com/VirusBLITZ/guess_the_song_backend/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/VirusBLITZ/guess_the_song_backend.svg?style=for-the-badge
[forks-url]: https://github.com/VirusBLITZ/guess_the_song_backend/network/members
[stars-shield]: https://img.shields.io/github/stars/VirusBLITZ/guess_the_song_backend.svg?style=for-the-badge
[stars-url]: https://github.com/VirusBLITZ/guess_the_song_backend/stargazers
[issues-shield]: https://img.shields.io/github/issues/VirusBLITZ/guess_the_song_backend.svg?style=for-the-badge
[issues-url]: https://github.com/VirusBLITZ/guess_the_song_backend/issues
[license-shield]: https://img.shields.io/github/license/VirusBLITZ/guess_the_song_backend.svg?style=for-the-badge
[license-url]: https://github.com/VirusBLITZ/guess_the_song_backend/blob/master/LICENSE.txt

[Rust]: https://img.shields.io/badge/rust-000000?style=for-the-badge&logo=rust&logoColor=orange
[Rust-url]: https://rust-lang.org/
