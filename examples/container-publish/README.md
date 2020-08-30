To run this example you'll need to provide credentials for Dockerhub and update the repository name.

First create a new file `.jarvis/secrets/config.secret.txt` and put a Docker config into it for authentication.

An easy tip for generating a valid one is to open an existing `~/.docker/config.json` file and set the `credsStore` key to an empty string.
Now you can run `docker login` again and you should get a warning, but also a plain `config.json` file which you can copy the contents of.
Don't forget to backup your original config file before doing this though!

Once you've got your credentials in place, you'll be able to run the build, but you won't be able to push to my Dockerhub, so you'll need
to update the `build.yaml`. Look for the push configuration `--output type=image,name=thetasinner/jarvis-sample-app,push=true` and replace _thetasinner_ with your Dockerhub username.
