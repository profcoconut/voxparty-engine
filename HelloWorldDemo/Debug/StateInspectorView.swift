import SwiftUI

struct StateInspectorView: View {
    let states: [StateVariable] = [
        StateVariable(name: "count", type: "Int", value: "42"),
        StateVariable(name: "isEnabled", type: "Bool", value: "true"),
        StateVariable(name: "userName", type: "String", value: "\"Alice\"")
    ]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                ForEach(states, id: \.name) { state in
                    HStack {
                        Image(systemName: "x.squareroot")
                            .font(.caption)
                            .foregroundColor(.purple)

                        Text(state.name)
                            .font(.system(.caption, design: .monospaced))
                            .fontWeight(.medium)

                        Spacer()

                        Text(state.type)
                            .font(.system(.caption2, design: .monospaced))
                            .foregroundColor(.secondary)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Color.secondary.opacity(0.2))
                            .cornerRadius(4)

                        Text(state.value)
                            .font(.system(.caption, design: .monospaced))
                            .foregroundColor(.green)
                    }
                    .padding(.vertical, 6)
                    .padding(.horizontal, 8)
                    .background(Color(UIColor.secondarySystemBackground))
                    .cornerRadius(6)
                }
            }
            .padding()
        }
    }
}

struct StateVariable {
    let name: String
    let type: String
    let value: String
}

#Preview {
    StateInspectorView()
}
