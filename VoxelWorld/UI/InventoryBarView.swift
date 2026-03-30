import SwiftUI

struct InventoryBarView: View {
    @ObservedObject var player: PlayerEntity

    var body: some View {
        HStack(spacing: 8) {
            ForEach(0..<5, id: \.self) { i in
                RoundedRectangle(cornerRadius: 4)
                    .fill(player.inventory[i].color)
                    .overlay(
                        RoundedRectangle(cornerRadius: 4)
                            .strokeBorder(Color.white, lineWidth: i == player.selectedSlot ? 3 : 1)
                    )
                    .overlay(
                        Text(slotLabel(for: i))
                            .font(.system(size: 10, weight: .bold))
                            .foregroundColor(.white)
                            .shadow(color: .black, radius: 1, x: 1, y: 1),
                        alignment: .bottomTrailing
                    )
                    .scaleEffect(i == player.selectedSlot ? 1.1 : 1.0)
                    .animation(.spring(duration: 0.2), value: player.selectedSlot)
                    .onTapGesture {
                        player.selectedSlot = i
                    }
            }
        }
        .padding(12)
        .background(Color.black.opacity(0.5))
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }

    private func slotLabel(for index: Int) -> String {
        switch index {
        case 0: return "1"
        case 1: return "2"
        case 2: return "3"
        case 3: return "4"
        case 4: return "5"
        default: return ""
        }
    }
}
