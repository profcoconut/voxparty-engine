import Foundation
import GameplayKit

class TerrainGenerator {
    static func generate(gridSize: Int) -> [[TileType]] {
        let noiseSource = GKPerlinNoiseSource(
            frequency: 4.0,
            octaveCount: 4,
            persistence: 0.5,
            lacunarity: 2.0,
            seed: Int32.random(in: 0..<Int32.max)
        )

        let noise = GKNoise(noiseSource)
        let noiseMap = GKNoiseMap(
            noise,
            size: double2(Double(gridSize), Double(gridSize)),
            origin: double2(0, 0),
            sampleCount: int2(Int32(gridSize), Int32(gridSize)),
            seamless: false
        )

        var world = [[TileType]]()

        for y in 0..<gridSize {
            var row = [TileType]()
            for x in 0..<gridSize {
                let value = noiseMap.value(at: int2(Int32(x), Int32(y)))
                let tileType = tileTypeForNoise(Double(value))
                row.append(tileType)
            }
            world.append(row)
        }

        return world
    }

    static func tileTypeForNoise(_ value: Double) -> TileType {
        switch value {
        case ..<(-0.3): return .water
        case (-0.3)..<(-0.1): return .water
        case (-0.1)..<(0.2): return .grass
        case (0.2)..<(0.6): return .grass
        case (0.6)..<(0.8): return .dirt
        default: return .stone
        }
    }
}
