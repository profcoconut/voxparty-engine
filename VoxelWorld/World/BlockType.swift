import SwiftUI

enum BlockType: Int, CaseIterable {
    case air = 0
    case grass = 1
    case dirt = 2
    case stone = 3
    case wood = 4
    case leaves = 5
    case water = 6

    var color: Color {
        switch self {
        case .air: return .clear
        case .grass: return Color(red: 0.3, green: 0.7, blue: 0.3)
        case .dirt: return Color(red: 0.55, green: 0.35, blue: 0.2)
        case .stone: return Color(red: 0.5, green: 0.5, blue: 0.55)
        case .wood: return Color(red: 0.55, green: 0.35, blue: 0.15)
        case .leaves: return Color(red: 0.2, green: 0.6, blue: 0.2)
        case .water: return Color(red: 0.2, green: 0.4, blue: 0.8)
        }
    }

    var isSolid: Bool {
        switch self {
        case .air, .water: return false
        default: return true
        }
    }
}
