import webpush from "web-push"

const { publicKey, privateKey } = webpush.generateVAPIDKeys()

console.log(`Public: ${publicKey}\nPrivate: ${privateKey}`)
