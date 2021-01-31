require("dotenv").config()

const fs = require("fs")
const path = require("path")
const { EnvironmentPlugin } = require("webpack")
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin")
const webpack = require("webpack")
const { ConcatSource } = require("webpack-sources")
const CopyPlugin = require("copy-webpack-plugin")

// Copying file with a newline to replicate behavior in wrangler/src/upload/form/mod.rs
class ConcatPlugin {
  apply(compiler) {
    const options = {}
    const matchObject = webpack.ModuleFilenameHelpers.matchObject.bind(
      undefined,
      options
    )
    compiler.hooks.compilation.tap("ConcatPlugin", (compilation) => {
      compilation.hooks.optimizeChunkAssets.tap("ConcatPlugin", (chunks) => {
        for (const chunk of chunks) {
          for (const file of chunk.files) {
            if (!matchObject(file)) {
              continue
            }

            const content = fs.readFileSync(
              path.resolve("./pkg", "index.js"),
              `utf8`
            )

            compilation.assets[file] = new ConcatSource(
              content,
              "\n",
              compilation.assets[file]
            )
          }
        }
      })
    })
  }
}

module.exports = {
  context: __dirname,
  mode: "production",
  target: "webworker",
  entry: {
    index: "./index.js",
  },
  optimization: {
    minimize: false,
    namedModules: true,
  },
  node: {
    net: "empty",
    tls: "empty",
  },
  plugins: [
    new WasmPackPlugin({
      crateDirectory: __dirname,
      // no-modules needed based off of wrangler default args
      extraArgs: "--target no-modules --no-typescript",
    }),
    new EnvironmentPlugin([
      "VAPID_PUBLIC_KEY",
      "VAPID_PRIVATE_KEY",
      "VAPID_SUBJECT",
    ]),
    new ConcatPlugin(),
    new CopyPlugin([{ from: "pkg/*.wasm", flatten: true }]),
  ],
}
