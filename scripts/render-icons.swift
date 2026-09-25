// アイコンのSVGを、512×512の透過PNGに書き出す（macOSのみ）。
// 影のフィルタまで描けるように、WebKitで描いてスナップショットを撮る。
//
// 使い方（リポジトリのルートで）: swift scripts/render-icons.swift

import AppKit
import WebKit

let size: CGFloat = 512
let icons = URL(fileURLWithPath: CommandLine.arguments[0])
    .deletingLastPathComponent()
    .deletingLastPathComponent()
    .appendingPathComponent("src-tauri/icons")
var queue = ["icon", "icon-dark"]

class Renderer: NSObject, WKNavigationDelegate {
    let web = WKWebView(frame: NSRect(x: 0, y: 0, width: size, height: size))
    var name = ""

    override init() {
        super.init()
        web.setValue(false, forKey: "drawsBackground")
        web.navigationDelegate = self
    }

    func next() {
        guard !queue.isEmpty else { exit(0) }
        name = queue.removeFirst()
        let svg = try! String(contentsOf: icons.appendingPathComponent("\(name).svg"), encoding: .utf8)
        let html = """
            <html><head><style>html,body{margin:0;background:transparent}\
            svg{display:block;width:\(Int(size))px;height:\(Int(size))px}</style></head><body>\(svg)</body></html>
            """
        web.loadHTMLString(html, baseURL: nil)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        let config = WKSnapshotConfiguration()
        config.rect = webView.bounds
        // スナップショットの幅はポイントで指定するので、画面の倍率で割ってピクセルを合わせる
        config.snapshotWidth = NSNumber(value: Double(size / (NSScreen.main?.backingScaleFactor ?? 2)))
        webView.takeSnapshot(with: config) { image, error in
            guard let image, let cg = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
                FileHandle.standardError.write("cannot render \(self.name).svg: \(String(describing: error))\n".data(using: .utf8)!)
                exit(1)
            }
            let out = icons.appendingPathComponent("\(self.name).png")
            try! NSBitmapImageRep(cgImage: cg).representation(using: .png, properties: [:])!.write(to: out)
            print("wrote \(out.path) (\(cg.width)x\(cg.height))")
            self.next()
        }
    }
}

let app = NSApplication.shared
let renderer = Renderer()
renderer.next()
app.run()
