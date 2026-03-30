import UIKit
import SpriteKit

class Textures {
    static let shared = Textures()

    var textures: [TileType: SKTexture] = [:]
    var size: CGSize = CGSize(width: 64, height: 32)

    private init() {
        for tileType in TileType.allCases {
            textures[tileType] = SKTexture(image: generateTile(type: tileType))
            textures[tileType]?.filteringMode = .nearest
        }
    }

    func generateTile(type: TileType) -> UIImage {
        let width = 64
        let height = 32
        var pixels = [UInt8](repeating: 0, count: width * height * 4)

        let (topColor, edgeColor) = colors(for: type)

        for py in 0..<height {
            let relY = py < height / 2 ? CGFloat(py) : CGFloat(height - 1 - py)
            let halfW = (relY / CGFloat(height - 1)) * CGFloat(width - 1) / 2
            let left = (width - 1) / 2 - Int(halfW)
            let right = (width - 1) / 2 + Int(halfW)

            for px in left...right {
                let idx = (py * width + px) * 4
                let isEdge = px == left || px == right || py == 0 || py == height - 1
                let color = isEdge ? edgeColor : topColor
                pixels[idx] = color.r
                pixels[idx + 1] = color.g
                pixels[idx + 2] = color.b
                pixels[idx + 3] = color.a
            }
        }

        return imageFromPixels(pixels, width: width, height: height)
    }

    struct RGBA { let r, g, b, a: UInt8 }

    func colors(for type: TileType) -> (top: RGBA, edge: RGBA) {
        switch type {
        case .grass: return (RGBA(r: 80, g: 160, b: 60, a: 255), RGBA(r: 40, g: 100, b: 30, a: 255))
        case .dirt: return (RGBA(r: 160, g: 110, b: 60, a: 255), RGBA(r: 120, g: 80, b: 40, a: 255))
        case .stone: return (RGBA(r: 140, g: 140, b: 150, a: 255), RGBA(r: 100, g: 100, b: 110, a: 255))
        case .wood: return (RGBA(r: 180, g: 120, b: 60, a: 255), RGBA(r: 140, g: 80, b: 40, a: 255))
        case .leaves: return (RGBA(r: 50, g: 140, b: 50, a: 255), RGBA(r: 30, g: 100, b: 30, a: 255))
        case .water: return (RGBA(r: 50, g: 100, b: 200, a: 180), RGBA(r: 30, g: 70, b: 170, a: 180))
        case .void: return (RGBA(r: 0, g: 0, b: 0, a: 0), RGBA(r: 0, g: 0, b: 0, a: 0))
        }
    }

    func imageFromPixels(_ pixels: [UInt8], width: Int, height: Int) -> UIImage {
        let colorSpace = CGColorSpaceCreateDeviceRGB()
        let bitmapInfo = CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue)

        guard let provider = CGDataProvider(data: Data(pixels) as CFData),
              let cgImage = CGImage(width: width, height: height, bitsPerComponent: 8,
                                   bitsPerPixel: 32, bytesPerRow: width * 4,
                                   space: colorSpace, bitmapInfo: bitmapInfo,
                                   provider: provider, decode: nil,
                                   shouldInterpolate: false, intent: .defaultIntent) else {
            return UIImage()
        }

        return UIImage(cgImage: cgImage)
    }

    func playerTexture() -> SKTexture {
        let size = 32
        var pixels = [UInt8](repeating: 0, count: size * size * 4)
        for y in 0..<size {
            for x in 0..<size {
                let isBorder = x == 0 || x == size-1 || y == 0 || y == size-1
                let color: RGBA = isBorder ? RGBA(r: 40, g: 80, b: 180, a: 255) : RGBA(r: 60, g: 120, b: 220, a: 255)
                let idx = (y * size + x) * 4
                pixels[idx] = color.r; pixels[idx+1] = color.g; pixels[idx+2] = color.b; pixels[idx+3] = color.a
            }
        }
        let img = imageFromPixels(pixels, width: size, height: size)
        let tex = SKTexture(image: img)
        tex.filteringMode = .nearest
        return tex
    }
}
